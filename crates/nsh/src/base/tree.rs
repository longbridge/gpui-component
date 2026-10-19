//! A control showing hierarchical items: Tree.

use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

use gpui_kit::{
    AppContext as _, ElementId, Entity, Focusable as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, StyleRefinement, Styled as _, Subscription,
    base::{
        StyledExt as _, TestSupportExt as _, Tree, TreeEvent, TreeItem, TreeSelectionEvent,
        TreeState,
    },
    div,
    prelude::FluentBuilder as _,
    px, svg,
};
use serde::Deserialize;
use serde_json::json;

use crate::{Control, Events, PropType, Props};

pub(super) fn controls() -> Vec<Control> {
    vec![tree()]
}

/// One node of the `items` prop.
#[derive(Deserialize)]
struct TreeNode {
    id: SharedString,
    label: SharedString,
    #[serde(default)]
    children: Vec<TreeNode>,
    #[serde(default)]
    disabled: bool,
}

impl TreeNode {
    fn into_item(self) -> TreeItem {
        TreeItem::new(self.id, self.label)
            .disabled(self.disabled)
            .children(self.children.into_iter().map(TreeNode::into_item))
    }
}

/// The interaction-state class props of each item. Items never take focus
/// themselves: the tree holds it.
const ITEM_STATES: [&str; 3] = ["itemHoverClass", "itemActiveClass", ""];

/// A tree's Base state and the items it shows, kept across frames.
struct TreeControl {
    state: Entity<TreeState>,
    /// The root items; clones share each item's expanded state with Base's.
    items: Rc<RefCell<Vec<TreeItem>>>,
    _subscriptions: [Subscription; 2],
}

/// Calls `f` with every item of `items` and their descendants, depth first.
fn walk(items: &[TreeItem], f: &mut impl FnMut(&TreeItem)) {
    for item in items {
        f(item);
        walk(&item.children, f);
    }
}

/// The ids of the expanded folders, depth first.
fn expanded_ids(items: &[TreeItem]) -> Vec<SharedString> {
    let mut ids = Vec::new();
    walk(items, &mut |item| {
        if item.is_folder() && item.is_expanded() {
            ids.push(item.id.clone());
        }
    });
    ids
}

impl TreeControl {
    /// Pushes the props JS set into Base's state.
    fn update(&self, props: &Props, events: &Events, cx: &mut gpui_kit::App) {
        if props.changed("items") || props.changed("expanded") {
            if props.changed("items") {
                *self.items.borrow_mut() = props
                    .json::<Vec<TreeNode>>("items")
                    .unwrap_or_default()
                    .into_iter()
                    .map(TreeNode::into_item)
                    .collect();
            }
            let expanded: HashSet<_> = props.strings("expanded").into_iter().collect();
            walk(&self.items.borrow(), &mut |item| {
                item.clone().expanded(expanded.contains(&item.id));
            });
            let items = self.items.borrow().clone();
            let cleared = self.state.update(cx, |state, cx| {
                let selected = state.selected_item().map(|item| item.id.clone());
                state.set_items(items, cx);
                // Keep the selection where it still shows.
                let ix = selected.as_ref().and_then(|id| state.index_of(id));
                state.set_selected_index(ix, cx);
                selected.is_some() && ix.is_none()
            });
            // The selected item went away with the items or its folder.
            if cleared {
                events.emit("select", json!({ "id": "" }));
            }
        }
        if props.changed("selected") {
            let id = props.string("selected");
            let mut item = None;
            if let Some(id) = &id {
                walk(&self.items.borrow(), &mut |candidate| {
                    if &candidate.id == id && item.is_none() {
                        item = Some(candidate.clone());
                    }
                });
            }
            // Base reveals a selected item inside collapsed folders.
            self.state
                .update(cx, |state, cx| state.set_selected_item(item.as_ref(), cx));
        }
    }
}

fn tree() -> Control {
    Control::stateful(
        "Tree",
        |_, events, _, cx| {
            let state = cx.new(|cx| TreeState::new(cx));
            let items = Rc::new(RefCell::new(Vec::<TreeItem>::new()));
            let expanded_change = cx.subscribe(&state, {
                let events = events.clone();
                let items = items.clone();
                move |_, _: &TreeEvent, _| {
                    let expanded = expanded_ids(&items.borrow());
                    events.emit("expandedChange", json!({ "expanded": expanded }));
                }
            });
            let select = cx.subscribe(&state, {
                let events = events.clone();
                move |_, event: &TreeSelectionEvent, _| {
                    events.emit("select", json!({ "id": event.id() }))
                }
            });
            TreeControl {
                state,
                items,
                _subscriptions: [expanded_change, select],
            }
        },
        |tree, props, events, window, cx| {
            tree.update(props, events, cx);
            let style = |name: &str| props.styled_by(name, StyleRefinement::default(), cx);
            let item_style = style("itemClass");
            let selected_style = style("selectedItemClass");
            let disabled_style = style("disabledItemClass");
            let disclosure_style = style("disclosureClass");
            let icon_style = style("iconClass");
            let label_style = style("labelClass");
            let item_states = props.state_styles(ITEM_STATES, cx);
            let indent = px(props.number("indent") as f32);
            let expand_icon = props.string("expandIcon");
            let collapse_icon = props.string("collapseIcon");
            let id = props.id();
            let focused = tree.state.read(cx).focus_handle(cx).is_focused(window);
            props
                .styled(Tree::new(&tree.state), cx)
                .when(focused, |this| props.styled_by("focusedClass", this, cx))
                .list_style(style("listClass"))
                .item(move |_, entry, entry_state, _, _| {
                    let item = entry.item();
                    let disabled = entry.is_disabled();
                    let icon = if !entry.is_folder() {
                        None
                    } else if entry.is_expanded() {
                        collapse_icon.clone()
                    } else {
                        expand_icon.clone()
                    };
                    div()
                        .id(ElementId::NamedChild(
                            Arc::new(id.clone()),
                            format!("item-{}", item.id).into(),
                        ))
                        .test_support()
                        .refine_style(&item_style)
                        .when(entry_state.is_selected(), |this| {
                            this.refine_style(&selected_style)
                        })
                        .when(disabled, |this| this.refine_style(&disabled_style))
                        .when(!disabled, |this| item_states.clone().apply(this))
                        .child(div().flex_none().w(indent * entry.depth() as f32))
                        .child(
                            div()
                                .refine_style(&disclosure_style)
                                .when_some(icon, |this, icon| {
                                    this.child(svg().path(icon).refine_style(&icon_style))
                                }),
                        )
                        .child(div().refine_style(&label_style).child(item.label.clone()))
                        .into_any_element()
                })
                .into_any_element()
        },
    )
    .focus(|tree: &TreeControl, cx| tree.state.focus_handle(cx))
    .documentation(
        "GPUI Base's Tree: a virtualized list of `items` and the children of the `expanded` \
         ones. A click selects an item and expands or collapses a folder; Up and Down move the \
         selection over the enabled items (wrapping around), Right expands and Left collapses the \
         selected folder, and Enter toggles it. `select` reports an `id` of \"\" when new \
         `items` or `expanded` hide the selected item. \
         Each item is a row (`itemClass`, a flex row) of an indent of `indent` pixels per depth, \
         a disclosure box holding `expandIcon` or `collapseIcon` for folders, and the label. A \
         virtual list draws only the rows its height holds: give `rootClass` a height and \
         `listClass` `size_full`.",
    )
    .declare(
        "export type TreeNode = { id: string; label: string; children?: TreeNode[]; disabled?: boolean };",
    )
    .prop("items", PropType::Json("TreeNode[]"))
    .prop("expanded", PropType::Strings)
    .prop("selected", PropType::String)
    .prop("indent", PropType::Number)
    .prop("expandIcon", PropType::String)
    .prop("collapseIcon", PropType::String)
    .prop("rootClass", PropType::String)
    .prop("focusedClass", PropType::String)
    .prop("listClass", PropType::String)
    .prop("itemClass", PropType::String)
    .states(ITEM_STATES)
    .prop("selectedItemClass", PropType::String)
    .prop("disabledItemClass", PropType::String)
    .prop("disclosureClass", PropType::String)
    .prop("iconClass", PropType::String)
    .prop("labelClass", PropType::String)
    .event("select", &[("id", PropType::String)])
    .event("expandedChange", &[("expanded", PropType::Strings)])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::{
        ElementId, TestAppContext,
        test::{TestQueryExt as _, TestWindowExt as _},
    };
    use serde_json::{Value, json};

    use crate::testing::Fixture;

    /// The id of the row of item `id`.
    fn row(fixture: &Fixture, id: &str) -> ElementId {
        ElementId::NamedChild(Arc::new(fixture.id.clone()), format!("item-{id}").into())
    }

    fn event(name: &str, data: Value) -> (String, Value) {
        (name.into(), data)
    }

    fn items() -> Value {
        json!([
            {
                "id": "src",
                "label": "src",
                "children": [
                    { "id": "src/lib.rs", "label": "lib.rs" },
                    { "id": "src/ui", "label": "ui", "children": [
                        { "id": "src/ui/tree.rs", "label": "tree.rs" },
                    ] },
                ],
            },
            { "id": "locked", "label": "locked", "disabled": true },
            { "id": "README.md", "label": "README.md" },
        ])
    }

    /// A focused tree of `items` and `props`. A virtual list draws only the
    /// rows its height holds, so the tree takes one.
    fn tree(cx: &mut TestAppContext, mut props: Value) -> Fixture {
        props["items"] = items();
        props["rootClass"] = json!("w_64 h_64");
        props["listClass"] = json!("size_full");
        props["itemClass"] = json!("flex h_6");
        let fixture = Fixture::new(cx, "Tree", props);
        let focus = fixture.focus_handle(cx).expect("a tree is a Tab stop");
        fixture.update(cx, |window, cx| focus.focus(window, cx));
        fixture
    }

    #[gpui_kit::test]
    fn test_tree_builder(cx: &mut TestAppContext) {
        let tree = Fixture::new(
            cx,
            "Tree",
            json!({
                "items": items(),
                "expanded": ["src"],
                "selected": "src/lib.rs",
                "indent": 16,
                "expandIcon": "icons/chevron-right.svg",
                "collapseIcon": "icons/chevron-down.svg",
                "rootClass": "w_64 h_64 p_1 border_1 border_border rounded_md",
                "focusedClass": "border_ring",
                "listClass": "size_full",
                "itemClass": "flex items_center gap_1 h_7 px_2 rounded_md text_sm",
                "itemHoverClass": "bg_muted",
                "itemActiveClass": "bg_muted",
                "selectedItemClass": "bg_accent text_accent_foreground",
                "disabledItemClass": "text_muted_foreground opacity_50",
                "disclosureClass": "size_4 flex_none flex items_center justify_center",
                "iconClass": "size_4 text_muted_foreground",
                "labelClass": "truncate",
            }),
        );
        let size = tree.measure(cx);
        assert!(size.width > gpui_kit::px(0.) && size.height > gpui_kit::px(0.));
        tree.update(cx, |window, _| {
            // The expanded folder shows its children, not its grandchildren.
            assert!(window.try_find(row(&tree, "src/ui")).is_some());
            assert!(window.try_find(row(&tree, "src/ui/tree.rs")).is_none());
            assert!(window.find_by_label("lib.rs").selected() == Some(true));
            assert_ne!(window.find_by_label("src").selected(), Some(true));
            assert_eq!(window.find_by_label("src").expanded(), Some(true));
            assert_eq!(window.find_by_label("ui").expanded(), Some(false));
            // Rows span the list; the indent is inside them.
            let parent = window.find(row(&tree, "src")).bounds();
            let child = window.find(row(&tree, "src/lib.rs")).bounds();
            assert_eq!(parent.origin.x, child.origin.x);
            assert!(child.size.height > gpui_kit::px(0.));
        });
        assert!(tree.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_tree_click(cx: &mut TestAppContext) {
        let tree = tree(cx, json!({}));
        tree.update(cx, |window, cx| window.click(row(&tree, "src"), cx));
        assert_eq!(
            tree.take_events(),
            vec![
                event("select", json!({ "id": "src" })),
                event("expandedChange", json!({ "expanded": ["src"] })),
            ]
        );
        tree.update(cx, |window, cx| window.click(row(&tree, "src/ui"), cx));
        assert_eq!(
            tree.take_events(),
            vec![
                event("select", json!({ "id": "src/ui" })),
                event("expandedChange", json!({ "expanded": ["src", "src/ui"] })),
            ]
        );
        tree.update(cx, |window, cx| {
            window.click(row(&tree, "src/ui/tree.rs"), cx)
        });
        assert_eq!(
            tree.take_events(),
            vec![event("select", json!({ "id": "src/ui/tree.rs" }))]
        );
        // Collapsing a folder hides its children.
        tree.update(cx, |window, cx| window.click(row(&tree, "src"), cx));
        assert_eq!(
            tree.take_events(),
            vec![
                event("select", json!({ "id": "src" })),
                event("expandedChange", json!({ "expanded": ["src/ui"] })),
            ]
        );
        tree.update(cx, |window, _| {
            assert!(window.try_find(row(&tree, "src/lib.rs")).is_none());
        });
    }

    #[gpui_kit::test]
    fn test_tree_keyboard(cx: &mut TestAppContext) {
        let tree = tree(cx, json!({}));
        tree.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(
            tree.take_events(),
            vec![event("select", json!({ "id": "src" }))]
        );
        tree.update(cx, |window, cx| window.press("right", cx));
        tree.update(cx, |window, cx| window.press("down", cx));
        assert_eq!(
            tree.take_events(),
            vec![
                event("expandedChange", json!({ "expanded": ["src"] })),
                event("select", json!({ "id": "src/lib.rs" })),
            ]
        );
        // Right and Left act on folders only.
        tree.update(cx, |window, cx| window.press("right", cx));
        tree.update(cx, |window, cx| window.press("up", cx));
        tree.update(cx, |window, cx| window.press("left", cx));
        assert_eq!(
            tree.take_events(),
            vec![
                event("select", json!({ "id": "src" })),
                event("expandedChange", json!({ "expanded": [] })),
            ]
        );
        tree.update(cx, |window, cx| window.press("enter", cx));
        assert_eq!(
            tree.take_events(),
            vec![event("expandedChange", json!({ "expanded": ["src"] }))]
        );
        // Up from the first item wraps to the last, past the disabled one.
        tree.update(cx, |window, cx| window.press("up", cx));
        tree.update(cx, |window, cx| window.press("up", cx));
        assert_eq!(
            tree.take_events(),
            vec![
                event("select", json!({ "id": "README.md" })),
                event("select", json!({ "id": "src/ui" })),
            ]
        );
    }

    #[gpui_kit::test]
    fn test_tree_controlled(cx: &mut TestAppContext) {
        let tree = tree(cx, json!({}));
        tree.set_prop(cx, "expanded", Some(json!(["src", "src/ui"])));
        tree.set_prop(cx, "selected", Some(json!("src/ui/tree.rs")));
        tree.update(cx, |window, _| {
            assert!(window.find_by_label("tree.rs").selected() == Some(true));
        });
        assert!(tree.take_events().is_empty());

        // Collapsing the selected item's folders clears the selection.
        tree.set_prop(cx, "expanded", Some(json!([])));
        tree.update(cx, |window, _| {
            assert!(window.try_find(row(&tree, "src/ui/tree.rs")).is_none());
        });
        assert_eq!(
            tree.take_events(),
            vec![event("select", json!({ "id": "" }))]
        );

        // Selecting a hidden item reveals it, which Base reports.
        tree.set_prop(cx, "selected", Some(json!("src/lib.rs")));
        assert_eq!(
            tree.take_events(),
            vec![event("expandedChange", json!({ "expanded": ["src"] }))]
        );

        // New items keep the selection while its item shows.
        tree.set_prop(
            cx,
            "items",
            Some(json!([
                { "id": "src", "label": "source", "children": [
                    { "id": "src/lib.rs", "label": "lib.rs" },
                ] },
            ])),
        );
        tree.set_prop(cx, "expanded", Some(json!(["src"])));
        tree.update(cx, |window, _| {
            assert!(window.find_by_label("lib.rs").selected() == Some(true));
            assert!(window.try_find_by_label("source").is_some());
        });
        assert!(tree.take_events().is_empty());

        // Collapsing the selected item's folder clears the selection.
        tree.set_prop(cx, "expanded", Some(json!([])));
        assert_eq!(
            tree.take_events(),
            vec![event("select", json!({ "id": "" }))]
        );
        tree.set_prop(cx, "expanded", Some(json!(["src"])));
        tree.update(cx, |window, _| {
            assert_ne!(window.find_by_label("lib.rs").selected(), Some(true));
        });
        // Clearing it from JS is JS's own change.
        tree.set_prop(cx, "selected", Some(json!("src/lib.rs")));
        tree.set_prop(cx, "selected", None);
        tree.update(cx, |window, _| {
            assert_ne!(window.find_by_label("lib.rs").selected(), Some(true));
        });
        assert!(tree.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_tree_disabled_item(cx: &mut TestAppContext) {
        let tree = tree(cx, json!({}));
        tree.update(cx, |window, cx| window.click(row(&tree, "locked"), cx));
        assert!(tree.take_events().is_empty());
        tree.update(cx, |window, _| {
            assert_ne!(window.find_by_label("locked").selected(), Some(true));
        });
    }
}
