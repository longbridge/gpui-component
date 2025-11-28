use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    App, Context, ElementId, Entity, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding,
    ListSizingBehavior, MouseButton, ParentElement, Render, RenderOnce, SharedString,
    StyleRefinement, Styled, UniformListScrollHandle, Window, div, prelude::FluentBuilder as _,
    uniform_list,
};

use crate::{
    StyledExt,
    actions::{Confirm, SelectDown, SelectLeft, SelectRight, SelectUp},
    list::ListItem,
    scroll::ScrollableElement,
};

const CONTEXT: &str = "Tree";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

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
///     let item = entry.item();
///     ListItem::new(ix).pl(px(16.) * entry.depth()).child(item.label.clone())
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
    pub fn children(mut self, children: impl IntoIterator<Item = TreeItem>) -> Self {
        self.children.extend(children);
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
    focus_handle: FocusHandle,
    entries: Vec<TreeEntry>,
    scroll_handle: UniformListScrollHandle,
    selected_ix: Option<usize>,
    render_item: Rc<dyn Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem>,
}

impl TreeState {
    /// Create a new empty tree state.
    pub fn new(cx: &mut App) -> Self {
        Self {
            selected_ix: None,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::default(),
            entries: Vec::new(),
            render_item: Rc::new(|_, _, _, _, _| ListItem::new(0)),
        }
    }

    /// Set the tree items.
    pub fn items(mut self, items: impl Into<Vec<TreeItem>>) -> Self {
        let items = items.into();
        self.entries.clear();
        for item in items.into_iter() {
            self.add_entry(item, 0);
        }
        self
    }

    /// Set the tree items.
    pub fn set_items(&mut self, items: impl Into<Vec<TreeItem>>, cx: &mut Context<Self>) {
        let items = items.into();
        self.entries.clear();
        for item in items.into_iter() {
            self.add_entry(item, 0);
        }
        self.selected_ix = None;
        cx.notify();
    }

    /// Get the currently selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_ix
    }

    /// Set the selected index, or `None` to clear selection.
    pub fn set_selected_index(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.selected_ix = ix;
        cx.notify();
    }

    pub fn scroll_to_item(&mut self, ix: usize, strategy: gpui::ScrollStrategy) {
        self.scroll_handle.scroll_to_item(ix, strategy);
    }

    /// Get the currently selected entry, if any.
    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.selected_ix.and_then(|ix| self.entries.get(ix))
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

    fn on_action_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                if entry.is_folder() {
                    self.toggle_expand(selected_ix);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                if entry.is_folder() && entry.is_expanded() {
                    self.toggle_expand(selected_ix);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                if entry.is_folder() && !entry.is_expanded() {
                    self.toggle_expand(selected_ix);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let mut selected_ix = self.selected_ix.unwrap_or(0);

        if selected_ix > 0 {
            selected_ix = selected_ix - 1;
        } else {
            selected_ix = self.entries.len().saturating_sub(1);
        }

        self.selected_ix = Some(selected_ix);
        self.scroll_handle
            .scroll_to_item(selected_ix, gpui::ScrollStrategy::Top);
        cx.notify();
    }

    fn on_action_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let mut selected_ix = self.selected_ix.unwrap_or(0);
        if selected_ix + 1 < self.entries.len() {
            selected_ix = selected_ix + 1;
        } else {
            selected_ix = 0;
        }

        self.selected_ix = Some(selected_ix);
        self.scroll_handle
            .scroll_to_item(selected_ix, gpui::ScrollStrategy::Bottom);
        cx.notify();
    }

    fn on_entry_click(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_ix = Some(ix);
        self.toggle_expand(ix);
        cx.notify();
    }
}

impl Render for TreeState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_item = self.render_item.clone();

        div().id("tree-state").size_full().relative().child(
            uniform_list("entries", self.entries.len(), {
                cx.processor(move |state, visible_range: Range<usize>, window, cx| {
                    let mut items = Vec::with_capacity(visible_range.len());
                    for ix in visible_range {
                        let entry = &state.entries[ix];
                        let selected = Some(ix) == state.selected_ix;
                        let item = (render_item)(ix, entry, selected, window, cx);

                        let el = div()
                            .id(ix)
                            .child(item.disabled(entry.item().is_disabled()).selected(selected))
                            .when(!entry.item().is_disabled(), |this| {
                                this.on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        move |this, _, window, cx| {
                                            this.on_entry_click(ix, window, cx);
                                        }
                                    }),
                                )
                            });

                        items.push(el)
                    }

                    items
                })
            })
            .flex_grow()
            .size_full()
            .track_scroll(self.scroll_handle.clone())
            .with_sizing_behavior(ListSizingBehavior::Auto)
            .into_any_element(),
        )
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
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let scroll_handle = self.state.read(cx).scroll_handle.clone();

        self.state
            .update(cx, |state, _| state.render_item = self.render_item);

        div()
            .id(self.id)
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TreeState::on_action_confirm))
            .on_action(window.listener_for(&self.state, TreeState::on_action_left))
            .on_action(window.listener_for(&self.state, TreeState::on_action_right))
            .on_action(window.listener_for(&self.state, TreeState::on_action_up))
            .on_action(window.listener_for(&self.state, TreeState::on_action_down))
            .size_full()
            .child(self.state)
            .refine_style(&self.style)
            .vertical_scrollbar(&scroll_handle)
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::TreeState;
    use gpui::AppContext as _;

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

    #[gpui::test]
    fn test_tree_entry(cx: &mut gpui::TestAppContext) {
        use super::TreeItem;

        let items = vec![
            TreeItem::new("src", "src")
                .expanded(true)
                .child(
                    TreeItem::new("src/ui", "ui")
                        .expanded(true)
                        .child(TreeItem::new("src/ui/button.rs", "button.rs"))
                        .child(TreeItem::new("src/ui/icon.rs", "icon.rs"))
                        .child(TreeItem::new("src/ui/mod.rs", "mod.rs")),
                )
                .child(TreeItem::new("src/lib.rs", "lib.rs")),
            TreeItem::new("Cargo.toml", "Cargo.toml"),
            TreeItem::new("Cargo.lock", "Cargo.lock").disabled(true),
            TreeItem::new("README.md", "README.md"),
        ];

        let state = cx.new(|cx| TreeState::new(cx).items(items));
        state.update(cx, |state, _| {
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
        })
    }
}
