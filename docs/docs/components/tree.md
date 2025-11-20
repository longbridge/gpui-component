---
title: Tree
description: A hierarchical tree view component for displaying and navigating tree-structured data.
---

# Tree

A versatile tree component for displaying hierarchical data with expand/collapse functionality, keyboard navigation, custom item rendering, and built-in context menu support. Perfect for file explorers, navigation menus, or any nested data structure.

## Import

```rust
use gpui_component::tree::{tree, TreeState, TreeItem, TreeEntry, TreeDelegate};
```

## Usage

### Basic Tree

```rust
// Create a delegate
struct BasicTreeDelegate;

impl TreeDelegate for BasicTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        ListItem::new(ix)
            .selected(selected)
            .child(
                h_flex()
                    .gap_2()
                    .child(entry.item().label.clone())
            )
    }
}

// Create tree state with delegate
let tree_state = cx.new(|cx| {
    TreeState::new(BasicTreeDelegate, cx).items(vec![
        TreeItem::new("src", "src")
            .expanded(true)
            .child(TreeItem::new("src/lib.rs", "lib.rs"))
            .child(TreeItem::new("src/main.rs", "main.rs")),
        TreeItem::new("Cargo.toml", "Cargo.toml"),
        TreeItem::new("README.md", "README.md"),
    ])
});

// Render tree
tree(&tree_state)
```

### File Tree with Icons and Context Menu

```rust
use gpui_component::{ListItem, IconName, h_flex, Menu, MenuItem};

struct FileTreeDelegate;

impl TreeDelegate for FileTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        let item = entry.item();
        let icon = if !entry.is_folder() {
            IconName::File
        } else if entry.is_expanded() {
            IconName::FolderOpen
        } else {
            IconName::Folder
        };

        ListItem::new(ix)
            .selected(selected)
            .pl(px(16.) * entry.depth() + px(12.)) // Indent based on depth
            .child(
                h_flex()
                    .gap_2()
                    .child(icon)
                    .child(item.label.clone())
            )
    }

    fn context_menu(
        &self,
        ix: usize,
        menu: gpui_component::menu::PopupMenu,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui_component::menu::PopupMenu {
        menu.label(format!("Selected Index: {}", ix))
    }
}

// Create tree state with the delegate
let tree_state = cx.new(|cx| {
    TreeState::new(FileTreeDelegate, cx).items(items)
});

// Render tree with context menu support
tree(&tree_state)
```

### Dynamic Tree Loading

```rust
struct DynamicTreeDelegate;

impl TreeDelegate for DynamicTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        ListItem::new(ix)
            .selected(selected)
            .child(entry.item().label.clone())
    }
}

impl MyView {
    fn load_files(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let tree_state = self.tree_state.clone();
        cx.spawn(async move |cx| {
            let items = build_file_items(&path).await;
            tree_state.update(cx, |state, cx| {
                state.set_items(items, cx);
            })
        }).detach();
    }
}

fn build_file_items(path: &Path) -> Vec<TreeItem> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            if path.is_dir() {
                let children = build_file_items(&path);
                items.push(TreeItem::new(path.to_string_lossy(), name)
                    .children(children));
            } else {
                items.push(TreeItem::new(path.to_string_lossy(), name));
            }
        }
    }
    items
}

// Create tree state
let tree_state = cx.new(|cx| {
    TreeState::new(DynamicTreeDelegate, cx)
});
```

### Tree with Selection Handling

```rust
struct MyTreeView {
    tree_state: Entity<TreeState<SelectionTreeDelegate>>,
    selected_item: Option<TreeItem>,
}

struct SelectionTreeDelegate {
    view: Entity<MyTreeView>,
}

impl TreeDelegate for SelectionTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        let item = entry.item();
        let view = self.view.clone();

        ListItem::new(ix)
            .selected(selected)
            .child(item.label.clone())
            .on_click(cx.listener({
                let item = item.clone();
                move |this, _, _, cx| {
                    view.update(cx, |view, cx| {
                        view.handle_selection(item.clone(), cx);
                    });
                }
            }))
    }
}

impl MyTreeView {
    fn new(cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| {
            TreeState::new(SelectionTreeDelegate { view: cx.entity() }, cx)
        });

        Self { tree_state, selected_item: None }
    }

    fn handle_selection(&mut self, item: TreeItem, cx: &mut Context<Self>) {
        self.selected_item = Some(item.clone());
        println!("Selected: {} ({})", item.label, item.id);
        cx.notify();
    }
}

// In render method
tree(&self.tree_state)
```

### Disabled Items

```rust
TreeItem::new("protected", "Protected Folder")
    .disabled(true)
    .child(TreeItem::new("secret.txt", "secret.txt"))
```

### Programmatic Tree Control

```rust
// Get current selection
if let Some(entry) = tree_state.read(cx).selected_entry() {
    println!("Current selection: {}", entry.item().label);
}

// Set selection programmatically
tree_state.update(cx, |state, cx| {
    state.set_selected_index(Some(2), cx); // Select third item
});

// Scroll to specific item
tree_state.update(cx, |state, _| {
    state.scroll_to_item(5, gpui::ScrollStrategy::Center);
});

// Clear selection
tree_state.update(cx, |state, cx| {
    state.set_selected_index(None, cx);
});
```

## API Reference

### TreeDelegate

The `TreeDelegate` trait defines how tree items are rendered and provides context menu functionality.

| Method                                         | Description                                                                     |
| ---------------------------------------------- | ------------------------------------------------------------------------------- |
| `render_item(ix, entry, selected, window, cx)` | Render a tree item as a `ListItem` (required)                                   |
| `context_menu(ix, menu, window, cx)`           | Customize context menu for tree item (optional, default returns menu unchanged) |

#### render_item Method Parameters

- `ix: usize`: Item index in flattened tree
- `entry: &TreeEntry`: Tree entry with item and depth metadata
- `selected: bool`: Whether item is currently selected
- `window: &mut Window`: Current window context
- `cx: &mut App`: Application context
- Returns: `ListItem` for rendering

### TreeState<D: TreeDelegate>

| Method                         | Description                         |
| ------------------------------ | ----------------------------------- |
| `new(delegate, cx)`            | Create new tree state with delegate |
| `items(items)`                 | Set initial tree items              |
| `set_items(items, cx)`         | Update tree items and notify        |
| `selected_index()`             | Get currently selected index        |
| `set_selected_index(ix, cx)`   | Set selected index                  |
| `selected_entry()`             | Get currently selected entry        |
| `delegate()`                   | Get reference to the delegate       |
| `scroll_to_item(ix, strategy)` | Scroll to specific item             |

### TreeItem

| Method            | Description                            |
| ----------------- | -------------------------------------- |
| `new(id, label)`  | Create new tree item with ID and label |
| `child(item)`     | Add single child item                  |
| `children(items)` | Add multiple child items               |
| `expanded(bool)`  | Set expanded state                     |
| `disabled(bool)`  | Set disabled state                     |
| `is_folder()`     | Check if item has children             |
| `is_expanded()`   | Check if item is expanded              |
| `is_disabled()`   | Check if item is disabled              |

### TreeEntry

| Method          | Description                 |
| --------------- | --------------------------- |
| `item()`        | Get the source TreeItem     |
| `depth()`       | Get item depth in tree      |
| `is_folder()`   | Check if entry has children |
| `is_expanded()` | Check if entry is expanded  |
| `is_disabled()` | Check if entry is disabled  |

### tree() Function

| Parameter | Description                                            |
| --------- | ------------------------------------------------------ |
| `state`   | `Entity<TreeState<D>>` for managing tree with delegate |

## Examples

### Lazy Loading Tree

```rust
struct LazyTreeView {
    tree_state: Entity<TreeState<LazyTreeDelegate>>,
    loaded_paths: HashSet<String>,
}

struct LazyTreeDelegate {
    view: Entity<LazyTreeView>,
}

impl TreeDelegate for LazyTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        let item = entry.item();
        let view = self.view.clone();

        ListItem::new(ix)
            .selected(selected)
            .pl(px(16.) * entry.depth() + px(12.))
            .child(item.label.clone())
            .on_click(window.listener_for(&self.view, {
                let item_id = item.id.clone();
                move |this, _, _, cx| {
                    view.update(cx, |view, cx| {
                        view.load_children(&item_id, cx);
                    });
                }
            }))
    }
}

impl LazyTreeView {
    fn new(cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| {
            TreeState::new(LazyTreeDelegate { view: cx.entity() }, cx)
        });

        Self { tree_state, loaded_paths: HashSet::new() }
    }

    fn load_children(&mut self, item_id: &str, cx: &mut Context<Self>) {
        if self.loaded_paths.contains(item_id) {
            return;
        }

        let path = PathBuf::from(item_id);
        if path.is_dir() {
            let tree_state = self.tree_state.clone();
            let item_id = item_id.to_string();

            cx.spawn(async move |cx| {
                let children = load_directory_children(&path).await;
                tree_state.update(cx, |state, cx| {
                    // Update tree with new items - you'd need to implement this logic
                    state.set_items(children, cx);
                })
            }).detach();

            self.loaded_paths.insert(item_id.to_string());
        }
    }
}
```

### Search and Filter

```rust
struct SearchableTree {
    tree_state: Entity<TreeState<SearchableTreeDelegate>>,
    original_items: Vec<TreeItem>,
    search_query: String,
}

struct SearchableTreeDelegate;

impl TreeDelegate for SearchableTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        ListItem::new(ix)
            .selected(selected)
            .pl(px(16.) * entry.depth() + px(12.))
            .child(entry.item().label.clone())
    }
}

impl SearchableTree {
    fn new(cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| {
            TreeState::new(SearchableTreeDelegate, cx)
        });

        Self {
            tree_state,
            original_items: Vec::new(),
            search_query: String::new(),
        }
    }

    fn filter_tree(&mut self, query: &str, cx: &mut Context<Self>) {
        self.search_query = query.to_string();

        let filtered_items = if query.is_empty() {
            self.original_items.clone()
        } else {
            filter_tree_items(&self.original_items, query)
        };

        self.tree_state.update(cx, |state, cx| {
            state.set_items(filtered_items, cx);
        });
    }
}

fn filter_tree_items(items: &[TreeItem], query: &str) -> Vec<TreeItem> {
    items.iter()
        .filter_map(|item| {
            if item.label.to_lowercase().contains(&query.to_lowercase()) {
                Some(item.clone().expanded(true)) // Auto-expand matches
            } else {
                // Check if any children match
                let filtered_children = filter_tree_items(&item.children, query);
                if !filtered_children.is_empty() {
                    Some(item.clone()
                        .children(filtered_children)
                        .expanded(true))
                } else {
                    None
                }
            }
        })
        .collect()
}
```

### Multi-Select Tree

```rust
struct MultiSelectTree {
    tree_state: Entity<TreeState<MultiSelectTreeDelegate>>,
    selected_items: HashSet<String>,
}

struct MultiSelectTreeDelegate {
    view: Entity<MultiSelectTree>,
}

impl TreeDelegate for MultiSelectTreeDelegate {
    fn render_item(
        &self,
        ix: usize,
        entry: &TreeEntry,
        selected: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> ListItem {
        let item = entry.item();
        let view = self.view.clone();
        let is_multi_selected = view.read(cx).is_selected(&item.id);

        ListItem::new(ix)
            .selected(is_multi_selected)
            .pl(px(16.) * entry.depth() + px(12.))
            .child(
                h_flex()
                    .gap_2()
                    .child(checkbox().checked(is_multi_selected))
                    .child(item.label.clone())
            )
            .on_click(window.listener_for(&self.view, {
                let item_id = item.id.clone();
                move |this, _, _, cx| {
                    view.update(cx, |view, cx| {
                        view.toggle_selection(&item_id, cx);
                    });
                }
            }))
    }
}

impl MultiSelectTree {
    fn new(cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| {
            TreeState::new(MultiSelectTreeDelegate { view: cx.entity() }, cx)
        });

        Self {
            tree_state,
            selected_items: HashSet::new(),
        }
    }

    fn toggle_selection(&mut self, item_id: &str, cx: &mut Context<Self>) {
        if self.selected_items.contains(item_id) {
            self.selected_items.remove(item_id);
        } else {
            self.selected_items.insert(item_id.to_string());
        }
        cx.notify();
    }

    fn is_selected(&self, item_id: &str) -> bool {
        self.selected_items.contains(item_id)
    }
}

// In render method
tree(&self.tree_state)
```

## Keyboard Navigation

The Tree component supports comprehensive keyboard navigation:

| Key     | Action                                    |
| ------- | ----------------------------------------- |
| `↑`     | Select previous item                      |
| `↓`     | Select next item                          |
| `←`     | Collapse current folder or move to parent |
| `→`     | Expand current folder                     |
| `Enter` | Toggle expand/collapse for folders        |
| `Space` | Custom action (configurable)              |

```rust
// Custom keyboard handling
tree(&tree_state)
    .key_context("MyTree")
    .on_action(cx.listener(|this, action: &MyCustomAction, _, cx| {
        // Handle custom actions
    }))
```
