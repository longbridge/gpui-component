---
title: Menu
description: Context menus and popup menus with support for icons, shortcuts, submenus, and various menu item types.
---

# PopupMenu

The Menu component provides both context menus (right-click menus) and popup menus with comprehensive features including icons, keyboard shortcuts, submenus, separators, checkable items, and custom elements. Built with accessibility and keyboard navigation in mind.

## Import

```rust
use gpui_component::{
    context_menu::ContextMenuExt,
    popup_menu::{PopupMenu, PopupMenuExt},
    Button
};
use gpui::{actions, Action};
```

## Usage

### Context Menu

Context menus appear when right-clicking on an element:

```rust
use gpui_component::context_menu::ContextMenuExt;

div()
    .child("Right click me")
    .context_menu(|menu, window, cx| {
        menu.menu("Copy", Box::new(Copy))
            .menu("Paste", Box::new(Paste))
            .separator()
            .menu("Delete", Box::new(Delete))
    })
```

### Popup Menu

Popup menus are triggered by buttons or other interactive elements:

```rust
use gpui_component::popup_menu::PopupMenuExt;

Button::new("menu-btn")
    .label("Open Menu")
    .popup_menu(|menu, window, cx| {
        menu.menu("New File", Box::new(NewFile))
            .menu("Open File", Box::new(OpenFile))
            .separator()
            .menu("Exit", Box::new(Exit))
    })
```

### Menu with Anchor Position

Control where the popup menu appears relative to the trigger:

```rust
use gpui::Corner;

Button::new("menu-btn")
    .label("Options")
    .popup_menu_with_anchor(Corner::TopRight, |menu, window, cx| {
        menu.menu("Option 1", Box::new(Action1))
            .menu("Option 2", Box::new(Action2))
    })
```

### Menu Items with Icons

Add icons to menu items for better visual clarity:

```rust
use gpui_component::IconName;

menu.menu_with_icon("Search", IconName::Search, Box::new(Search))
    .menu_with_icon("Settings", IconName::Settings, Box::new(OpenSettings))
    .separator()
    .menu_with_icon("Help", IconName::Help, Box::new(ShowHelp))
```

### Checkable Menu Items

Create menu items that show a check state:

```rust
let is_enabled = true;

menu.menu_with_check("Enable Feature", is_enabled, Box::new(ToggleFeature))
    .menu_with_check("Show Sidebar", sidebar_visible, Box::new(ToggleSidebar))
```

### Menu Items with Keyboard Shortcuts

Menu items automatically display keyboard shortcuts if they're bound to actions:

```rust
// First define your actions and key bindings
actions!(my_app, [Copy, Paste, Cut]);

// In your app initialization
cx.bind_keys([
    KeyBinding::new("ctrl-c", Copy, Some("editor")),
    KeyBinding::new("ctrl-v", Paste, Some("editor")),
    KeyBinding::new("ctrl-x", Cut, Some("editor")),
]);

// The menu will automatically show shortcuts
menu.action_context(focus_handle) // Set context for shortcuts
    .menu("Copy", Box::new(Copy))     // Will show "Ctrl+C"
    .menu("Paste", Box::new(Paste))   // Will show "Ctrl+V"
    .menu("Cut", Box::new(Cut))       // Will show "Ctrl+X"
```

### Submenus

Create nested menus with submenu support:

```rust
menu.submenu("File", window, cx, |submenu, window, cx| {
        submenu.menu("New", Box::new(NewFile))
            .menu("Open", Box::new(OpenFile))
            .separator()
            .menu("Recent", Box::new(ShowRecent))
    })
    .submenu("Edit", window, cx, |submenu, window, cx| {
        submenu.menu("Undo", Box::new(Undo))
            .menu("Redo", Box::new(Redo))
    })
```

### Submenus with Icons

Add icons to submenu headers:

```rust
menu.submenu_with_icon(
        Some(IconName::Folder.into()),
        "Project",
        window,
        cx,
        |submenu, window, cx| {
            submenu.menu("Open Project", Box::new(OpenProject))
                .menu("Close Project", Box::new(CloseProject))
        }
    )
```

### Disabled Menu Items

Create disabled menu items that cannot be activated:

```rust
menu.menu("Available Action", Box::new(Action1))
    .menu_with_disabled("Disabled Action", Box::new(Action2), true)
    .menu_with_icon_and_disabled(
        "Unavailable",
        IconName::Lock,
        Box::new(Action3),
        true
    )
```

### Menu Separators

Use separators to group related menu items:

```rust
menu.menu("New", Box::new(NewFile))
    .menu("Open", Box::new(OpenFile))
    .separator()  // Groups file operations
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
    .separator()  // Groups edit operations
    .menu("Exit", Box::new(Exit))
```

### Menu Labels

Add non-interactive labels to organize menu sections:

```rust
menu.label("File Operations")
    .menu("New", Box::new(NewFile))
    .menu("Open", Box::new(OpenFile))
    .separator()
    .label("Edit Operations")
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
```

### Link Menu Items

Create menu items that open external links:

```rust
menu.link("Documentation", "https://docs.example.com")
    .link_with_icon(
        "GitHub",
        IconName::GitHub,
        "https://github.com/example/repo"
    )
    .separator()
    .external_link_icon(false) // Hide external link icons
    .link("Support", "https://support.example.com")
```

### Custom Menu Elements

Create custom menu items with complex content:

```rust
use gpui_component::{h_flex, v_flex};

menu.menu_element(Box::new(CustomAction), |window, cx| {
        v_flex()
            .child("Custom Element")
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("This is a subtitle")
            )
    })
    .menu_element_with_icon(
        IconName::Info,
        Box::new(InfoAction),
        |window, cx| {
            h_flex()
                .gap_1()
                .child("Status")
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().success)
                        .child("✓ Connected")
                )
        }
    )
```

### Scrollable Menus

For menus with many items, enable scrolling:

```rust
Button::new("large-menu")
    .label("Many Options")
    .popup_menu(|menu, window, cx| {
        let mut menu = menu
            .scrollable()
            .max_h(px(300.))
            .label("Select an option");

        for i in 0..100 {
            menu = menu.menu(
                format!("Option {}", i),
                Box::new(SelectOption(i))
            );
        }
        menu
    })
```

### Menu Sizing

Control menu dimensions:

```rust
menu.min_w(px(200.))      // Minimum width
    .max_w(px(400.))      // Maximum width
    .max_h(px(300.))      // Maximum height
    .scrollable()         // Enable scrolling when content exceeds max height
```

### Action Context

Set the focus context for handling menu actions:

```rust
let focus_handle = cx.focus_handle();

menu.action_context(focus_handle)
    .menu("Copy", Box::new(Copy))
    .menu("Paste", Box::new(Paste))
```

## API Reference

### PopupMenu

| Method                     | Description                                   |
| -------------------------- | --------------------------------------------- |
| `build(window, cx, f)`     | Create a new popup menu with builder function |
| `action_context(handle)`   | Set focus handle for action dispatch          |
| `min_w(width)`             | Set minimum width                             |
| `max_w(width)`             | Set maximum width                             |
| `max_h(height)`            | Set maximum height                            |
| `scrollable()`             | Enable vertical scrolling                     |
| `external_link_icon(bool)` | Show/hide external link icons                 |

### Menu Items

| Method                                        | Description                       |
| --------------------------------------------- | --------------------------------- |
| `menu(label, action)`                         | Add basic menu item               |
| `menu_with_disabled(label, action, disabled)` | Add menu item with disabled state |
| `menu_with_icon(label, icon, action)`         | Add menu item with icon           |
| `menu_with_check(label, checked, action)`     | Add checkable menu item           |
| `label(text)`                                 | Add non-interactive label         |
| `separator()`                                 | Add visual separator              |
| `link(label, url)`                            | Add link menu item                |
| `link_with_icon(label, icon, url)`            | Add link with icon                |
| `item(item)`                                  | Add custom menu item              |

### Custom Elements

| Method                                              | Description                         |
| --------------------------------------------------- | ----------------------------------- |
| `menu_element(action, builder)`                     | Add custom element menu item        |
| `menu_element_with_icon(icon, action, builder)`     | Add custom element with icon        |
| `menu_element_with_check(checked, action, builder)` | Add custom element with check state |

### Submenus

| Method                                | Description |
| ------------------------------------- | ----------- |
| `submenu(label, window, cx, builder)` | Add submenu |

### PopupMenuExt Trait

| Method                                    | Description                                |
| ----------------------------------------- | ------------------------------------------ |
| `popup_menu(builder)`                     | Add popup menu to button (top-left anchor) |
| `popup_menu_with_anchor(corner, builder)` | Add popup menu with custom anchor          |

### ContextMenuExt Trait

| Method                  | Description                 |
| ----------------------- | --------------------------- |
| `context_menu(builder)` | Add context menu to element |

## Examples

### File Manager Context Menu

```rust
div()
    .child("Right-click for options")
    .context_menu(|menu, window, cx| {
        menu.menu_with_icon("Open", IconName::FolderOpen, Box::new(Open))
            .separator()
            .menu_with_icon("Copy", IconName::Copy, Box::new(Copy))
            .menu_with_icon("Cut", IconName::Scissors, Box::new(Cut))
            .menu_with_icon("Paste", IconName::Clipboard, Box::new(Paste))
            .separator()
            .submenu("New", window, cx, |submenu, window, cx| {
                submenu.menu_with_icon("File", IconName::File, Box::new(NewFile))
                    .menu_with_icon("Folder", IconName::Folder, Box::new(NewFolder))
            })
            .separator()
            .menu_with_icon("Delete", IconName::Trash, Box::new(Delete))
            .separator()
            .menu("Properties", Box::new(ShowProperties))
    })
```

### Add MenuItem without action

Sometimes you may not like to define an action for a menu item, you just want add a `on_click` handler, in this case, the `item` and `PopupMenuItem` can help you:

```rust
use gpui_component::{popup_menu::PopupMenuItem, Button};

Button::new("custom-item-menu")
    .label("Options")
    .popup_menu(|menu, window, cx| {
        menu.item(
            PopupMenuItem::new("Custom Action")
                .disabled(false)
                .icon(IconName::Star)
                .on_click(|window, cx| {
                    // Custom click handler logic
                    println!("Custom Action Clicked!");
                })
        )
        .separator()
        .menu("Standard Action", Box::new(StandardAction))
    })
```

### Editor Menu with Shortcuts

```rust
// Define actions with keyboard shortcuts
actions!(editor, [Save, SaveAs, Find, Replace, ToggleWordWrap]);

// Set up key bindings
cx.bind_keys([
    KeyBinding::new("ctrl-s", Save, Some("editor")),
    KeyBinding::new("ctrl-shift-s", SaveAs, Some("editor")),
    KeyBinding::new("ctrl-f", Find, Some("editor")),
    KeyBinding::new("ctrl-h", Replace, Some("editor")),
]);

// Create menu with automatic shortcuts
let editor_focus = cx.focus_handle();

Button::new("editor-menu")
    .label("Edit")
    .popup_menu(|menu, window, cx| {
        menu.action_context(editor_focus)
            .menu("Save", Box::new(Save))           // Shows "Ctrl+S"
            .menu("Save As...", Box::new(SaveAs))   // Shows "Ctrl+Shift+S"
            .separator()
            .menu("Find", Box::new(Find))           // Shows "Ctrl+F"
            .menu("Replace", Box::new(Replace))     // Shows "Ctrl+H"
            .separator()
            .menu_with_check("Word Wrap", true, Box::new(ToggleWordWrap))
    })
```

### Settings Menu with Custom Elements

```rust
Button::new("settings")
    .label("Settings")
    .popup_menu(|menu, window, cx| {
        menu.label("Display")
            .menu_element_with_check(dark_mode, Box::new(ToggleDarkMode), |window, cx| {
                h_flex()
                    .gap_2()
                    .child("Dark Mode")
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if dark_mode { "On" } else { "Off" })
                    )
            })
            .separator()
            .label("Account")
            .menu_element_with_icon(
                IconName::User,
                Box::new(ShowProfile),
                |window, cx| {
                    v_flex()
                        .child("John Doe")
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("john@example.com")
                        )
                }
            )
            .separator()
            .link_with_icon("Help Center", IconName::Help, "https://help.example.com")
            .menu("Sign Out", Box::new(SignOut))
    })
```

## Keyboard Shortcuts

| Key               | Action                            |
| ----------------- | --------------------------------- |
| `↑` / `↓`         | Navigate menu items               |
| `←` / `→`         | Navigate submenus                 |
| `Enter` / `Space` | Activate menu item                |
| `Escape`          | Close menu                        |
| `Tab`             | Close menu and focus next element |

## Best Practices

1. **Group Related Items**: Use separators to group related functionality
2. **Consistent Icons**: Use consistent iconography across your application
3. **Logical Order**: Place most common actions at the top
4. **Keyboard Shortcuts**: Provide shortcuts for frequently used actions
5. **Context Awareness**: Show only relevant items for the current context
6. **Progressive Disclosure**: Use submenus for complex hierarchies
7. **Clear Labels**: Use descriptive, action-oriented labels
8. **Reasonable Limits**: Use scrollable menus for more than 10-15 items
