---
title: Tabs
description: A set of layered sections of content—known as tab panels—that are displayed one at a time.
---

# Tabs

A tabbed interface component for organizing content into separate sections. Supports multiple variants, sizes, navigation controls, and interactive features like reordering and prefix/suffix elements.

## Import

```rust
use gpui_component::tab::{Tab, TabBar};
```

## Usage

### Basic Tabs

```rust
TabBar::new("tabs")
    .selected_index(0)
    .on_click(|selected_index, _, _| {
        println!("Tab {} selected", selected_index);
    })
    .child(Tab::new("Account"))
    .child(Tab::new("Profile"))
    .child(Tab::new("Settings"))
```

### Tab Variants

#### Default Tabs

```rust
TabBar::new("default-tabs")
    .selected_index(0)
    .child(Tab::new("Account"))
    .child(Tab::new("Profile"))
    .child(Tab::new("Documents"))
```

#### Underline Tabs

```rust
TabBar::new("underline-tabs")
    .underline()
    .selected_index(0)
    .child("Account")
    .child("Profile")
    .child("Documents")
```

#### Pill Tabs

```rust
TabBar::new("pill-tabs")
    .pill()
    .selected_index(0)
    .child(Tab::new("Account"))
    .child(Tab::new("Profile"))
    .child(Tab::new("Documents"))
```

#### Outline Tabs

```rust
TabBar::new("outline-tabs")
    .outline()
    .selected_index(0)
    .child(Tab::new("Account"))
    .child(Tab::new("Profile"))
    .child(Tab::new("Documents"))
```

#### Segmented Tabs

```rust
use gpui_component::IconName;

TabBar::new("segmented-tabs")
    .segmented()
    .selected_index(0)
    .child(IconName::Bot)
    .child(IconName::Calendar)
    .child(IconName::Map)
    .children(vec!["Settings", "About"])
```

### Tab Sizes

```rust
// Extra Small
TabBar::new("tabs").xsmall()
    .child(Tab::new("Small"))

// Small
TabBar::new("tabs").small()
    .child(Tab::new("Small"))

// Medium (default)
TabBar::new("tabs")
    .child(Tab::new("Medium"))

// Large
TabBar::new("tabs").large()
    .child(Tab::new("Large"))
```

### Tabs with Icons

```rust
use gpui_component::{Icon, IconName};

TabBar::new("icon-tabs")
    .child(Tab::icon(IconName::User).with_variant(TabVariant::Tab))
    .child(Tab::icon(IconName::Settings).with_variant(TabVariant::Tab))
    .child(Tab::icon(IconName::Mail).with_variant(TabVariant::Tab))
```

### Tabs with Prefix and Suffix

```rust
use gpui_component::button::Button;
use gpui_component::{h_flex, IconName};

TabBar::new("tabs-with-controls")
    .prefix(
        h_flex()
            .gap_1()
            .child(Button::new("back").ghost().xsmall().icon(IconName::ArrowLeft))
            .child(Button::new("forward").ghost().xsmall().icon(IconName::ArrowRight))
    )
    .suffix(
        h_flex()
            .gap_1()
            .child(Button::new("inbox").ghost().xsmall().icon(IconName::Inbox))
            .child(Button::new("more").ghost().xsmall().icon(IconName::Ellipsis))
    )
    .child(Tab::new("Account"))
    .child(Tab::new("Profile"))
    .child(Tab::new("Settings"))
```

### Disabled Tabs

```rust
TabBar::new("tabs-with-disabled")
    .child(Tab::new("Account"))
    .child(Tab::new("Profile").disabled(true))
    .child(Tab::new("Settings"))
```

### Dynamic Tabs

```rust
struct TabsView {
    active_tab: usize,
    tabs: Vec<String>,
}

impl Render for TabsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("dynamic-tabs")
            .selected_index(self.active_tab)
            .on_click(cx.listener(|view, index, _, cx| {
                view.active_tab = *index;
                cx.notify();
            }))
            .children(
                self.tabs
                    .iter()
                    .map(|tab_name| Tab::new(tab_name.clone()))
            )
    }
}
```

### Tabs with Menu

```rust
TabBar::new("tabs-with-menu")
    .with_menu(true)
    .selected_index(0)
    .child(Tab::new("Account"))
    .child(Tab::new("Profile"))
    .child(Tab::new("Documents"))
    .child(Tab::new("Mail"))
    .child(Tab::new("Settings"))
```

### Scrollable Tabs

```rust
use gpui::ScrollHandle;

struct ScrollableTabsView {
    scroll_handle: ScrollHandle,
}

impl Render for ScrollableTabsView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("scrollable-tabs")
            .track_scroll(&self.scroll_handle)
            .child(Tab::new("Very Long Tab Name 1"))
            .child(Tab::new("Very Long Tab Name 2"))
            .child(Tab::new("Very Long Tab Name 3"))
            .child(Tab::new("Very Long Tab Name 4"))
            .child(Tab::new("Very Long Tab Name 5"))
    }
}
```

### Individual Tab Configuration

```rust
TabBar::new("custom-tabs")
    .child(
        Tab::new("Custom Tab")
            .id("custom-id")
            .prefix(IconName::Star)
            .suffix(IconName::X)
            .on_click(|_, _, _| {
                println!("Custom tab clicked");
            })
    )
```

## API Reference

### TabBar

| Method                      | Description                                        |
| --------------------------- | -------------------------------------------------- |
| `new(id)`                   | Create a new tab bar with the given ID             |
| `child(tab)`                | Add a tab to the bar                               |
| `children(tabs)`            | Add multiple tabs to the bar                       |
| `selected_index(index)`     | Set the active tab index                           |
| `on_click(fn)`              | Callback when a tab is clicked, receives tab index |
| `prefix(element)`           | Add element before the tabs                        |
| `suffix(element)`           | Add element after the tabs                         |
| `last_empty_space(element)` | Custom element for empty space at the end          |
| `track_scroll(handle)`      | Enable scrolling with a scroll handle              |
| `with_menu(bool)`           | Enable dropdown menu for tab selection             |

### TabBar Variants

| Method                  | Description                          |
| ----------------------- | ------------------------------------ |
| `with_variant(variant)` | Set the tab variant for all children |
| `underline()`           | Use underline variant                |
| `pill()`                | Use pill variant                     |
| `outline()`             | Use outline variant                  |
| `segmented()`           | Use segmented variant                |

### Tab

| Method                  | Description                                    |
| ----------------------- | ---------------------------------------------- |
| `new(label)`            | Create a new tab with a label                  |
| `empty()`               | Create an empty tab                            |
| `icon(icon)`            | Create a tab with only an icon                 |
| `id(id)`                | Set custom ID for the tab                      |
| `with_variant(variant)` | Set the tab variant                            |
| `pill()`                | Use pill variant                               |
| `outline()`             | Use outline variant                            |
| `segmented()`           | Use segmented variant                          |
| `underline()`           | Use underline variant                          |
| `prefix(element)`       | Add element before tab content                 |
| `suffix(element)`       | Add element after tab content                  |
| `disabled(bool)`        | Set disabled state                             |
| `selected(bool)`        | Set selected state (usually handled by TabBar) |
| `on_click(fn)`          | Custom click handler for individual tab        |

### TabVariant

```rust
pub enum TabVariant {
    Tab,      // Default bordered tabs
    Outline,  // Rounded outline tabs
    Pill,     // Rounded pill-shaped tabs
    Segmented, // Segmented control style
    Underline, // Underline indicator tabs
}
```

### Styling

Both `TabBar` and `Tab` implement `Sizable` trait:

- `xsmall()` - Extra small size
- `small()` - Small size
- `medium()` - Medium size (default)
- `large()` - Large size

## Advanced Examples

### Custom Tab Content

```rust
Tab::empty()
    .child(
        h_flex()
            .items_center()
            .gap_2()
            .child(IconName::Folder)
            .child("Documents")
            .child(
                div()
                    .px_1()
                    .py_0p5()
                    .text_xs()
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
                    .rounded_sm()
                    .child("12")
            )
    )
```

### Tabs with State Management

```rust
struct TabsWithContent {
    active_tab: usize,
    tab_contents: Vec<String>,
}

impl TabsWithContent {
    fn render_tab_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.active_tab {
            0 => div().child("Account content"),
            1 => div().child("Profile content"),
            2 => div().child("Settings content"),
            _ => div().child("Unknown content"),
        }
    }
}

impl Render for TabsWithContent {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .child(
                TabBar::new("content-tabs")
                    .selected_index(self.active_tab)
                    .on_click(cx.listener(|view, index, _, cx| {
                        view.active_tab = *index;
                        cx.notify();
                    }))
                    .child(Tab::new("Account"))
                    .child(Tab::new("Profile"))
                    .child(Tab::new("Settings"))
            )
            .child(
                div()
                    .flex_1()
                    .p_4()
                    .child(self.render_tab_content(cx))
            )
    }
}
```

### Tabs with Close Buttons

While the basic Tab component doesn't include closeable functionality, you can create closeable tabs using suffix elements:

```rust
struct CloseableTabsView {
    tabs: Vec<String>,
    active_tab: usize,
}

impl CloseableTabsView {
    fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.tabs.len() > 1 {
            self.tabs.remove(index);
            if self.active_tab >= index && self.active_tab > 0 {
                self.active_tab -= 1;
            }
            cx.notify();
        }
    }
}

impl Render for CloseableTabsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("closeable-tabs")
            .selected_index(self.active_tab)
            .on_click(cx.listener(|view, index, _, cx| {
                view.active_tab = *index;
                cx.notify();
            }))
            .children(
                self.tabs
                    .iter()
                    .enumerate()
                    .map(|(index, tab_name)| {
                        Tab::new(tab_name.clone())
                            .suffix(
                                Button::new(format!("close-{}", index))
                                    .icon(IconName::X)
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.close_tab(index, cx);
                                    }))
                            )
                    })
            )
    }
}
```

## Accessibility

- Keyboard navigation with Tab and Arrow keys
- Enter or Space to activate tabs
- Focus indicators for keyboard navigation
- Disabled tabs cannot be focused or activated
- Screen reader support for tab labels and states
- ARIA attributes for tab panels and tab lists
- Proper focus management when tabs are added or removed

## Notes

- The `TabBar` manages the selection state of all child tabs
- Individual tab `on_click` handlers are ignored when `TabBar.on_click` is set
- Tabs automatically inherit the variant and size from their parent `TabBar`
- The `with_menu` option adds a dropdown for tab selection when there are many tabs
- Scrolling is automatically enabled when tabs overflow the container width
- The dock system provides advanced closeable tab functionality for complex layouts
