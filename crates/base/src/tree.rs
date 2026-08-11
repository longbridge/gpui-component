use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Context, ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement,
    Interactivity, IntoElement, KeyBinding, ListSizingBehavior, MouseButton, ParentElement, Render,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    UniformListScrollHandle, Window, div, prelude::FluentBuilder as _, uniform_list,
};

use crate::actions::{Confirm, SelectDown, SelectLeft, SelectRight, SelectUp};

const CONTEXT: &str = "Tree";

#[doc(hidden)]
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

#[doc(hidden)]
pub const fn key_context() -> &'static str {
    CONTEXT
}

struct TreeItemState {
    expanded: bool,
    disabled: bool,
}

/// A tree item with a stable id, display label, children, and shared state.
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
    pub fn new(item: TreeItem, depth: usize) -> Self {
        Self { item, depth }
    }

    #[inline]
    pub fn item(&self) -> &TreeItem {
        &self.item
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.depth == 0
    }

    #[inline]
    pub fn is_folder(&self) -> bool {
        self.item.is_folder()
    }

    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.item.is_expanded()
    }

    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.item.is_disabled()
    }
}

/// Event emitted by a tree when user-visible expansion state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    Expanded(SharedString),
    Collapsed(SharedString),
}

impl TreeItem {
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

    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = TreeItem>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn expanded(self, expanded: bool) -> Self {
        self.state.borrow_mut().expanded = expanded;
        self
    }

    pub fn disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self
    }

    #[inline]
    pub fn is_folder(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.state.borrow().expanded
    }

    /// Returns the target's ancestors from nearest parent to root.
    pub fn ancestors(&self, target_id: &SharedString) -> Option<Vec<TreeItem>> {
        if self.id == *target_id {
            return Some(Vec::new());
        }

        for child in &self.children {
            if let Some(mut path) = child.ancestors(target_id) {
                path.push(self.clone());
                return Some(path);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_state_and_ancestors_keep_nearest_first_order() {
        let leaf = TreeItem::new("leaf", "Leaf");
        let branch = TreeItem::new("branch", "Branch").child(leaf.clone());
        let root = TreeItem::new("root", "Root").child(branch.clone());

        leaf.clone().disabled(true).expanded(true);
        assert!(leaf.is_disabled());
        assert!(leaf.is_expanded());

        let ancestors = root.ancestors(&"leaf".into()).unwrap();
        assert_eq!(
            ancestors
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["branch", "root"]
        );
    }
}
