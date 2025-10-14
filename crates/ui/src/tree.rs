use gpui::{ElementId, SharedString};

use crate::{Disableable, Icon};

/// A tree item with a label, children, and an expanded state.
#[derive(Clone)]
pub struct TreeItem {
    pub label: SharedString,
    pub icon: Option<Icon>,
    pub children: Vec<TreeItem>,
    pub expanded: bool,
    pub disabled: bool,
}

/// A flat representation of a tree item with its depth.
#[derive(Clone)]
struct TreeEntry {
    item: TreeItem,
    depth: usize,
}

impl TreeItem {
    /// Create a new tree item with the given label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            children: Vec::new(),
            expanded: false,
            disabled: false,
        }
    }

    /// Add a child item to this tree item.
    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple child items to this tree item.
    pub fn children(mut self, children: Vec<TreeItem>) -> Self {
        self.children.extend(children);
        self
    }

    /// Set icon for this tree item.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set expanded state for this tree item.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set disabled state for this tree item.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct Tree {
    id: ElementId,
    pub items: Vec<TreeItem>,
    entries: Vec<TreeEntry>,
}

impl Tree {
    /// Create a new tree with the given root items.
    pub fn new(id: impl Into<ElementId>, items: Vec<TreeItem>) -> Self {
        let mut tree = Self {
            id: id.into(),
            items,
            entries: Vec::new(),
        };
        tree.rebuild_entries();
        tree
    }

    /// Rebuild the flat entries list from the hierarchical items.
    fn rebuild_entries(&mut self) {
        self.entries.clear();
        for item in self.items.clone().into_iter() {
            self.add_entry(item, 0);
        }
    }

    /// Recursively add entries to the flat list.
    fn add_entry(&mut self, item: TreeItem, depth: usize) {
        self.entries.push(TreeEntry {
            item: item.clone(),
            depth,
        });
        if item.expanded {
            for child in &item.children {
                self.add_entry(child.clone(), depth + 1);
            }
        }
    }

    /// Toggle the expanded state of a tree item at the given index.
    pub fn toggle_expanded(&mut self, index: usize) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.item.expanded = !entry.item.expanded;
            self.rebuild_entries();
        }
    }
}
