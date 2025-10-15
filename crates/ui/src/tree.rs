use std::{cell::RefCell, rc::Rc};

use gpui::{
    div, prelude::FluentBuilder as _, uniform_list, App, Context, ElementId, Entity,
    InteractiveElement as _, IntoElement, ListSizingBehavior, MouseButton, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, UniformListScrollHandle, Window,
};

use crate::{
    scroll::{Scrollbar, ScrollbarState},
    ListItem, StyledExt,
};

/// Create a [`Tree`].
///
/// # Arguments
///
/// * `state` - The shared state managing the tree items.
/// * `render_item` - A closure to render each tree item.
///
/// ```ignore
/// let state = cx.new(|_| {
///     TreeState::new().items(vec![
///         TreeItem::new("src")
///             .child(TreeItem::new("lib.rs"),
///         TreeItem::new("Cargo.toml"),
///         TreeItem::new("README.md"),
///     ])
/// });
///
/// tree(&state, |ix, entry, selected, window, cx| {
///     div().px(px(16.) * entry.depth()).child(item.label.clone())
/// })
/// ```
pub fn tree<R>(state: &Entity<TreeState>, render_item: R) -> Tree
where
    R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
{
    Tree::new(state, render_item)
}

struct TreeItemState {
    expanded: bool,
    disabled: bool,
}

/// A tree item with a label, children, and an expanded state.
#[derive(Clone)]
pub struct TreeItem {
    pub id: SharedString,
    pub label: SharedString,
    pub children: Vec<TreeItem>,
    state: Rc<RefCell<TreeItemState>>,
}

/// A flat representation of a tree item with its depth.
#[derive(Clone)]
pub struct TreeEntry {
    item: TreeItem,
    depth: usize,
}

impl TreeEntry {
    /// Get the source tree item.
    #[inline]
    pub fn item(&self) -> &TreeItem {
        &self.item
    }

    /// The depth of this item in the tree.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    fn is_root(&self) -> bool {
        self.depth == 0
    }

    /// Whether this item is a folder (has children).
    #[inline]
    pub fn is_folder(&self) -> bool {
        self.item.is_folder()
    }

    /// Return true if the item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.item.is_expanded()
    }

    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.item.is_disabled()
    }
}

impl TreeItem {
    /// Create a new tree item with the given label.
    ///
    /// - The `id` for you to uniquely identify this item, then later you can use it for selection or other purposes.
    /// - The `label` is the text to display for this item.
    ///
    /// For example, the `id` is the full file path, and the `label` is the file name.
    ///
    /// ```ignore
    /// TreeItem::new("src/ui/button.rs", "button.rs")
    /// ```
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            state: Rc::new(RefCell::new(TreeItemState {
                expanded: false,
                disabled: false,
            })),
        }
    }

    /// Add a child item to this tree item.
    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple child items to this tree item.
    pub fn children(mut self, children: impl Into<Vec<TreeItem>>) -> Self {
        self.children.extend(children.into());
        self
    }

    /// Set expanded state for this tree item.
    pub fn expanded(self, expanded: bool) -> Self {
        self.state.borrow_mut().expanded = expanded;
        self
    }

    /// Set disabled state for this tree item.
    pub fn disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self
    }

    /// Whether this item is a folder (has children).
    #[inline]
    pub fn is_folder(&self) -> bool {
        self.children.len() > 0
    }

    /// Return true if the item is disabled.
    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    /// Return true if the item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.state.borrow().expanded
    }
}

/// State for managing tree items.
pub struct TreeState {
    entries: Vec<TreeEntry>,
    scrollbar_state: ScrollbarState,
    scroll_handle: UniformListScrollHandle,
    selected_ix: Option<usize>,
}

impl TreeState {
    /// Create a new empty tree state.
    pub fn new() -> Self {
        Self {
            selected_ix: None,
            scrollbar_state: ScrollbarState::default(),
            scroll_handle: UniformListScrollHandle::default(),
            entries: Vec::new(),
        }
    }

    pub fn items(mut self, items: impl Into<Vec<TreeItem>>) -> Self {
        let items = items.into();
        self.entries.clear();
        for item in items.into_iter() {
            self.add_entry(item, 0);
        }
        self
    }

    pub fn set_items(&mut self, items: impl Into<Vec<TreeItem>>, cx: &mut Context<Self>) {
        let items = items.into();
        self.entries.clear();
        for item in items.into_iter() {
            self.add_entry(item, 0);
        }
        self.selected_ix = None;
        cx.notify();
    }

    fn add_entry(&mut self, item: TreeItem, depth: usize) {
        self.entries.push(TreeEntry {
            item: item.clone(),
            depth,
        });
        if item.is_expanded() {
            for child in &item.children {
                self.add_entry(child.clone(), depth + 1);
            }
        }
    }

    fn toggle_expand(&mut self, ix: usize) {
        let Some(entry) = self.entries.get_mut(ix) else {
            return;
        };
        if !entry.is_folder() {
            return;
        }

        entry.item.state.borrow_mut().expanded = !entry.is_expanded();
        self.rebuild_entries();
    }

    fn rebuild_entries(&mut self) {
        let root_items: Vec<TreeItem> = self
            .entries
            .iter()
            .filter(|e| e.is_root())
            .map(|e| e.item.clone())
            .collect();
        self.entries.clear();
        for item in root_items.into_iter() {
            self.add_entry(item, 0);
        }
    }
}

/// A tree view element that displays hierarchical data.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    state: Entity<TreeState>,
    style: StyleRefinement,
    render_item: Rc<dyn Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem>,
}

impl Tree {
    pub fn new<R>(state: &Entity<TreeState>, render_item: R) -> Self
    where
        R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
    {
        Self {
            id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            render_item: Rc::new(move |ix, item, selected, window, app| {
                render_item(ix, item, selected, window, app)
            }),
        }
    }

    fn on_entry_click(state: &Entity<TreeState>, ix: usize, _: &mut Window, cx: &mut App) {
        state.update(cx, |state, cx| {
            state.selected_ix = Some(ix);
            state.toggle_expand(ix);
            cx.notify();
        })
    }
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tree {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tree_state = self.state.read(cx);
        let render_item = self.render_item.clone();

        div()
            .id(self.id)
            .size_full()
            .child(
                uniform_list("items", tree_state.entries.len(), {
                    let selected_ix = tree_state.selected_ix;
                    let entries = tree_state.entries.clone();
                    let state = self.state.clone();
                    move |visible_range, window, cx| {
                        let mut items = Vec::with_capacity(visible_range.len());
                        for ix in visible_range {
                            let entry = &entries[ix];
                            let selected = Some(ix) == selected_ix;
                            let item = (render_item)(ix, entry, selected, window, cx);

                            let el = div()
                                .id(ix)
                                .child(item.disabled(entry.item().is_disabled()).selected(selected))
                                .when(!entry.item().is_disabled(), |this| {
                                    this.on_mouse_down(MouseButton::Left, {
                                        let state = state.clone();
                                        move |_, window, cx| {
                                            Self::on_entry_click(&state, ix, window, cx);
                                        }
                                    })
                                });

                            items.push(el)
                        }

                        items
                    }
                })
                .flex_grow()
                .size_full()
                .track_scroll(tree_state.scroll_handle.clone())
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .into_any_element(),
            )
            .refine_style(&self.style)
            .relative()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(Scrollbar::width())
                    .child(Scrollbar::vertical(
                        &tree_state.scrollbar_state,
                        &tree_state.scroll_handle,
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::TreeState;

    fn assert_entries(entries: &Vec<super::TreeEntry>, expected: &str) {
        let actual: Vec<String> = entries
            .iter()
            .map(|e| {
                let mut s = String::new();
                s.push_str(&"    ".repeat(e.depth));
                s.push_str(e.item().label.as_str());
                s
            })
            .collect();
        let actual = actual.join("\n");
        assert_eq!(actual.trim(), expected.trim());
    }

    #[test]
    fn test_tree_entry() {
        use super::TreeItem;

        let items = vec![
            TreeItem::new("src")
                .expanded(true)
                .child(
                    TreeItem::new("ui")
                        .expanded(true)
                        .child(TreeItem::new("button.rs"))
                        .child(TreeItem::new("icon.rs"))
                        .child(TreeItem::new("mod.rs")),
                )
                .child(TreeItem::new("lib.rs")),
            TreeItem::new("Cargo.toml"),
            TreeItem::new("Cargo.lock").disabled(true),
            TreeItem::new("README.md"),
        ];

        let mut state = TreeState::new().items(items);
        assert_entries(
            &state.entries,
            indoc! {
                r#"
                src
                    ui
                        button.rs
                        icon.rs
                        mod.rs
                    lib.rs
                Cargo.toml
                Cargo.lock
                README.md
                "#
            },
        );

        let entry = state.entries.get(0).unwrap();
        assert_eq!(entry.depth(), 0);
        assert_eq!(entry.is_root(), true);
        assert_eq!(entry.is_folder(), true);
        assert_eq!(entry.is_expanded(), true);

        let entry = state.entries.get(1).unwrap();
        assert_eq!(entry.depth(), 1);
        assert_eq!(entry.is_root(), false);
        assert_eq!(entry.is_folder(), true);
        assert_eq!(entry.is_expanded(), true);
        assert_eq!(entry.item().label.as_str(), "ui");

        state.toggle_expand(1);
        let entry = state.entries.get(1).unwrap();
        assert_eq!(entry.is_expanded(), false);
        assert_entries(
            &state.entries,
            indoc! {
                r#"
                src
                    ui
                    lib.rs
                Cargo.toml
                Cargo.lock
                README.md
                "#
            },
        );
    }
}
