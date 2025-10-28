---
title: Sidebar
description: A composable, themeable and customizable sidebar component for navigation and content organization.
---

# Sidebar

A flexible sidebar component that provides navigation structure for applications. Features collapsible states, nested menu items, header and footer sections, and responsive design. Perfect for creating application navigation panels, admin dashboards, and complex hierarchical interfaces.

## Import

```rust
use gpui_component::sidebar::{
    Sidebar, SidebarHeader, SidebarFooter, SidebarGroup,
    SidebarMenu, SidebarMenuItem, SidebarToggleButton
};
```

## Usage

### Basic Sidebar

```rust
use gpui_component::{sidebar::*, Side};

Sidebar::new(Side::Left)
    .header(
        SidebarHeader::new()
            .child("My Application")
    )
    .child(
        SidebarGroup::new("Navigation")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Dashboard")
                            .icon(IconName::LayoutDashboard)
                            .on_click(|_, _, _| println!("Dashboard clicked"))
                    )
                    .child(
                        SidebarMenuItem::new("Settings")
                            .icon(IconName::Settings)
                            .on_click(|_, _, _| println!("Settings clicked"))
                    )
            )
    )
    .footer(
        SidebarFooter::new()
            .child("User Profile")
    )
```

### Collapsible Sidebar

```rust
let mut collapsed = false;

Sidebar::new(Side::Left)
    .collapsed(collapsed)
    .collapsible(true)
    .header(
        SidebarHeader::new()
            .child(
                h_flex()
                    .child(Icon::new(IconName::Home))
                    .when(!collapsed, |this| this.child("Home"))
            )
    )
    .child(
        SidebarGroup::new("Menu")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Files")
                            .icon(IconName::Folder)
                    )
            )
    )

// Toggle button
SidebarToggleButton::left()
    .collapsed(collapsed)
    .on_click(|_, _, _| {
        collapsed = !collapsed;
    })
```

### Nested Menu Items

```rust
SidebarMenuItem::new("Projects")
    .icon(IconName::FolderOpen)
    .active(true)
    .children([
        SidebarMenuItem::new("Web App")
            .active(false)
            .on_click(|_, _, _| println!("Web App selected")),
        SidebarMenuItem::new("Mobile App")
            .active(true)
            .on_click(|_, _, _| println!("Mobile App selected")),
        SidebarMenuItem::new("Desktop App")
            .on_click(|_, _, _| println!("Desktop App selected")),
    ])
    .on_click(|_, _, _| {
        // Toggle project group
    })
```

### Multiple Groups

```rust
Sidebar::new(Side::Left)
    .child(
        SidebarGroup::new("Main")
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("Dashboard").icon(IconName::Home))
                    .child(SidebarMenuItem::new("Analytics").icon(IconName::BarChart))
            )
    )
    .child(
        SidebarGroup::new("Content")
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("Posts").icon(IconName::FileText))
                    .child(SidebarMenuItem::new("Media").icon(IconName::Image))
                    .child(SidebarMenuItem::new("Comments").icon(IconName::MessageCircle))
            )
    )
    .child(
        SidebarGroup::new("Settings")
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("General").icon(IconName::Settings))
                    .child(SidebarMenuItem::new("Users").icon(IconName::Users))
            )
    )
```

### With Badges and Suffixes

```rust
use gpui_component::{Badge, Switch};

SidebarMenuItem::new("Notifications")
    .icon(IconName::Bell)
    .suffix(
        Badge::new()
            .count(5)
            .child("5")
    )

SidebarMenuItem::new("Dark Mode")
    .icon(IconName::Moon)
    .suffix(
        Switch::new("dark-mode")
            .checked(true)
            .xsmall()
    )

SidebarMenuItem::new("Settings")
    .icon(IconName::Settings)
    .suffix(IconName::ChevronRight)
```

### Right-Side Placement

```rust
Sidebar::new(Side::Right)
    .width(300)
    .header(
        SidebarHeader::new()
            .child("Right Panel")
    )
    .child(
        SidebarGroup::new("Tools")
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("Inspector").icon(IconName::Search))
                    .child(SidebarMenuItem::new("Console").icon(IconName::Terminal))
            )
    )
```

### Custom Width and Styling

```rust
Sidebar::new(Side::Left)
    .width(280)  // Custom width in pixels
    .border_width(2)  // Custom border width
    .header(
        SidebarHeader::new()
            .p_4()  // Custom padding
            .rounded(cx.theme().radius)
            .child("Custom Styled Sidebar")
    )
```

### Interactive Header with Popup Menu

```rust
use gpui_component::popup_menu::PopupMenuExt;

SidebarHeader::new()
    .child(
        h_flex()
            .gap_2()
            .child(Icon::new(IconName::Building))
            .child("Company Name")
            .child(Icon::new(IconName::ChevronsUpDown))
    )
    .popup_menu(|menu, _, _| {
        menu.menu("Acme Corp", Box::new(SelectCompany("acme")))
            .menu("Tech Solutions", Box::new(SelectCompany("tech")))
            .separator()
            .menu("Switch Organization", Box::new(SwitchOrg))
    })
```

### Footer with User Information

```rust
SidebarFooter::new()
    .justify_between()
    .child(
        h_flex()
            .gap_2()
            .child(Icon::new(IconName::User))
            .when(!collapsed, |this| {
                this.child(
                    v_flex()
                        .child("John Doe")
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("john@example.com"))
                )
            })
    )
    .when(!collapsed, |this| {
        this.child(Icon::new(IconName::MoreHorizontal))
    })
```

### Responsive Sidebar

```rust
let is_mobile = window_width < 768;

Sidebar::new(Side::Left)
    .collapsed(is_mobile || manually_collapsed)
    .width(if is_mobile { 60 } else { 240 })
    .header(
        SidebarHeader::new()
            .child(
                div()
                    .when(!is_mobile, |this| this.child("Full App Name"))
                    .when(is_mobile, |this| this.child(Icon::new(IconName::Menu)))
            )
    )
```

## API Reference

### Sidebar

| Method              | Description                                         |
| ------------------- | --------------------------------------------------- |
| `new(side)`         | Create a sidebar on the specified side (Left/Right) |
| `left()`            | Create a left-side sidebar                          |
| `right()`           | Create a right-side sidebar                         |
| `width(px)`         | Set sidebar width (default: 255px)                  |
| `border_width(px)`  | Set border width (default: 1px)                     |
| `collapsible(bool)` | Make sidebar collapsible (default: true)            |
| `collapsed(bool)`   | Set collapsed state                                 |
| `header(element)`   | Set header content                                  |
| `footer(element)`   | Set footer content                                  |
| `child(element)`    | Add child element (must implement Collapsible)      |
| `children(iter)`    | Add multiple children                               |

### SidebarHeader

| Method           | Description                 |
| ---------------- | --------------------------- |
| `new()`          | Create a new sidebar header |
| `selected(bool)` | Set selected state          |
| `child(element)` | Add child element           |

Implements: `Selectable`, `Collapsible`, `ParentElement`, `Styled`, `InteractiveElement`, `PopupMenuExt`

### SidebarFooter

| Method           | Description                 |
| ---------------- | --------------------------- |
| `new()`          | Create a new sidebar footer |
| `selected(bool)` | Set selected state          |
| `child(element)` | Add child element           |

Implements: `Selectable`, `Collapsible`, `ParentElement`, `Styled`, `InteractiveElement`, `PopupMenuExt`

### SidebarGroup

| Method            | Description                 |
| ----------------- | --------------------------- |
| `new(label)`      | Create a group with a label |
| `child(element)`  | Add child element           |
| `children(iter)`  | Add multiple children       |
| `collapsed(bool)` | Set collapsed state         |

### SidebarMenu

| Method            | Description             |
| ----------------- | ----------------------- |
| `new()`           | Create a new menu       |
| `child(item)`     | Add menu item           |
| `children(iter)`  | Add multiple menu items |
| `collapsed(bool)` | Set collapsed state     |

### SidebarMenuItem

| Method            | Description                              |
| ----------------- | ---------------------------------------- |
| `new(label)`      | Create a menu item with label            |
| `icon(icon)`      | Set icon                                 |
| `active(bool)`    | Set active state                         |
| `on_click(fn)`    | Set click handler                        |
| `children(iter)`  | Add submenu items                        |
| `suffix(element)` | Add suffix element (badge, switch, etc.) |
| `collapsed(bool)` | Set collapsed state                      |

### SidebarToggleButton

| Method            | Description                     |
| ----------------- | ------------------------------- |
| `left()`          | Create toggle for left sidebar  |
| `right()`         | Create toggle for right sidebar |
| `side(side)`      | Set sidebar side                |
| `collapsed(bool)` | Set collapsed state             |
| `on_click(fn)`    | Set click handler               |

## Theming

The sidebar uses dedicated theme colors:

```rust
// Theme colors used by sidebar
cx.theme().sidebar                    // Background
cx.theme().sidebar_foreground         // Text color
cx.theme().sidebar_border            // Border color
cx.theme().sidebar_accent            // Hover/active background
cx.theme().sidebar_accent_foreground // Hover/active text
cx.theme().sidebar_primary           // Primary elements
cx.theme().sidebar_primary_foreground // Primary text
```

### Custom Theme Colors

```json
{
  "sidebar.background": "#fafafa",
  "sidebar.foreground": "#171717",
  "sidebar.border": "#e5e5e5",
  "sidebar.accent.background": "#e5e5e5",
  "sidebar.accent.foreground": "#171717",
  "sidebar.primary.background": "#171717",
  "sidebar.primary.foreground": "#fafafa"
}
```

## Examples

### File Explorer Sidebar

```rust
Sidebar::new(Side::Left)
    .header(
        SidebarHeader::new()
            .child(
                h_flex()
                    .gap_2()
                    .child(IconName::Folder)
                    .child("Explorer")
            )
    )
    .child(
        SidebarGroup::new("Folders")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("src")
                            .icon(IconName::FolderOpen)
                            .active(true)
                            .children([
                                SidebarMenuItem::new("components")
                                    .icon(IconName::Folder),
                                SidebarMenuItem::new("utils")
                                    .icon(IconName::Folder),
                                SidebarMenuItem::new("main.rs")
                                    .icon(IconName::FileCode)
                                    .active(true),
                            ])
                    )
                    .child(
                        SidebarMenuItem::new("tests")
                            .icon(IconName::Folder)
                    )
                    .child(
                        SidebarMenuItem::new("Cargo.toml")
                            .icon(IconName::FileText)
                    )
            )
    )
```

### Admin Dashboard Sidebar

```rust
Sidebar::new(Side::Left)
    .header(
        SidebarHeader::new()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .size_8()
                            .rounded_full()
                            .bg(cx.theme().primary)
                            .child(Icon::new(IconName::Crown))
                    )
                    .child("Admin Panel")
            )
    )
    .child(
        SidebarGroup::new("Overview")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Dashboard")
                            .icon(IconName::LayoutDashboard)
                            .active(true)
                    )
                    .child(
                        SidebarMenuItem::new("Analytics")
                            .icon(IconName::TrendingUp)
                            .suffix(Badge::new().count(2))
                    )
            )
    )
    .child(
        SidebarGroup::new("Management")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Users")
                            .icon(IconName::Users)
                            .suffix("1,234")
                    )
                    .child(
                        SidebarMenuItem::new("Orders")
                            .icon(IconName::ShoppingCart)
                            .suffix(Badge::new().dot().variant_destructive())
                    )
                    .child(
                        SidebarMenuItem::new("Products")
                            .icon(IconName::Package)
                    )
            )
    )
    .footer(
        SidebarFooter::new()
            .child(
                h_flex()
                    .gap_2()
                    .child(IconName::User)
                    .child("Administrator")
            )
            .child(IconName::LogOut)
    )
```

### Settings Sidebar

```rust
Sidebar::new(Side::Left)
    .width(300)
    .header(
        SidebarHeader::new()
            .child("Settings")
    )
    .child(
        SidebarGroup::new("General")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Appearance")
                            .icon(IconName::Palette)
                            .active(true)
                    )
                    .child(
                        SidebarMenuItem::new("Notifications")
                            .icon(IconName::Bell)
                            .suffix(
                                Switch::new("notifications")
                                    .checked(true)
                                    .xsmall()
                            )
                    )
                    .child(
                        SidebarMenuItem::new("Privacy")
                            .icon(IconName::Shield)
                    )
            )
    )
    .child(
        SidebarGroup::new("Advanced")
            .child(
                SidebarMenu::new()
                    .child(
                        SidebarMenuItem::new("Developer")
                            .icon(IconName::Code)
                            .children([
                                SidebarMenuItem::new("Debug Mode")
                                    .suffix(
                                        Switch::new("debug")
                                            .checked(false)
                                            .xsmall()
                                    ),
                                SidebarMenuItem::new("Console")
                                    .on_click(|_, _, _| println!("Open console")),
                            ])
                    )
                    .child(
                        SidebarMenuItem::new("Performance")
                            .icon(IconName::Zap)
                    )
            )
    )
```
