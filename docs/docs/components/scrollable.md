---
title: Scrollable
description: Scrollable container with custom scrollbars, scroll tracking, and virtualization support.
---

# Scrollable

A comprehensive scrollable container component that provides custom scrollbars, scroll tracking, and virtualization capabilities. Supports both vertical and horizontal scrolling with customizable appearance and behavior.

## Import

```rust
use gpui_component::{
    scroll::{Scrollable, ScrollbarState, ScrollbarAxis, ScrollbarShow},
    StyledExt as _,
};
```

## Usage

### Basic Scrollable Container

The simplest way to make any element scrollable is using the `scrollable()` method from `StyledExt`:

```rust
use gpui::{div, Axis};

div()
    .size_full()
    .child("Your content here")
    .scrollable(Axis::Vertical)
```

### Scrollable with Content

```rust
v_flex()
    .gap_2()
    .p_4()
    .child("Scrollable Content")
    .children((0..100).map(|i| {
        div()
            .h(px(40.))
            .w_full()
            .bg(cx.theme().secondary)
            .child(format!("Item {}", i))
    }))
    .scrollable(Axis::Vertical)
```

### Horizontal Scrolling

```rust
h_flex()
    .gap_2()
    .p_4()
    .children((0..50).map(|i| {
        div()
            .min_w(px(120.))
            .h(px(80.))
            .bg(cx.theme().accent)
            .child(format!("Card {}", i))
    }))
    .scrollable(Axis::Horizontal)
```

### Both Directions

```rust
div()
    .size_full()
    .child(
        div()
            .w(px(2000.))  // Wide content
            .h(px(2000.))  // Tall content
            .bg(cx.theme().background)
            .child("Large content area")
    )
    .scrollable(ScrollbarAxis::Both)
```

## Custom Scrollbars

### Manual Scrollbar Creation

For more control, you can create scrollbars manually:

```rust
use gpui_component::scroll::{Scrollbar, ScrollbarState};

pub struct ScrollableView {
    scroll_state: ScrollbarState,
    scroll_handle: ScrollHandle,
}

impl Render for ScrollableView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("content")
                    .track_scroll(&self.scroll_handle)
                    .overflow_scroll()
                    .size_full()
                    .child("Your scrollable content")
            )
            .child(
                Scrollbar::vertical(&self.scroll_state, &self.scroll_handle)
            )
    }
}
```

### Customizing Scrollbar Behavior

```rust
Scrollbar::both(&scroll_state, &scroll_handle)
    .axis(ScrollbarAxis::Vertical)
    .scroll_size(size(px(1000.), px(2000.))) // Custom content size
```

## Scroll Tracking

### ScrollbarState Management

The `ScrollbarState` tracks scrollbar visibility, hover states, and drag interactions:

```rust
use gpui_component::scroll::ScrollbarState;

pub struct MyView {
    scroll_state: ScrollbarState,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            scroll_state: ScrollbarState::default(),
        }
    }
}
```

### Responding to Scroll Events

```rust
// In your render method
div()
    .on_scroll_wheel(|view, event, _, cx| {
        // Handle scroll wheel events
        if event.delta.y != px(0.) {
            println!("Scrolled vertically: {:?}", event.delta.y);
        }
    })
    .scrollable(Axis::Vertical)
```

### Programmatic Scrolling

```rust
// Using ScrollHandle for programmatic control
impl MyView {
    fn scroll_to_top(&mut self) {
        self.scroll_handle.set_offset(point(px(0.), px(0.)));
    }

    fn scroll_to_bottom(&mut self) {
        let max_offset = self.scroll_handle.max_offset();
        self.scroll_handle.set_offset(point(px(0.), max_offset.y));
    }
}
```

## Virtualization

### VirtualList for Large Datasets

For rendering large lists efficiently, use `VirtualList`:

```rust
use gpui_component::{VirtualList, VirtualListScrollHandle};

pub struct LargeListView {
    items: Vec<String>,
    scroll_handle: VirtualListScrollHandle,
}

impl Render for LargeListView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item_count = self.items.len();

        VirtualList::new(
            self.scroll_handle.clone(),
            item_count,
            |ix, window, cx| {
                // Item sizes - can be different for each item
                size(px(300.), px(40.))
            },
            |ix, bounds, selected, window, cx| {
                // Render each item
                div()
                    .size(bounds.size)
                    .bg(if selected {
                        cx.theme().accent
                    } else {
                        cx.theme().background
                    })
                    .child(format!("Item {}: {}", ix, self.items[ix]))
                    .into_any_element()
            },
        )
    }
}
```

### Scrolling to Specific Items

```rust
impl LargeListView {
    fn scroll_to_item(&mut self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Top);
    }

    fn scroll_to_item_centered(&mut self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Center);
    }
}
```

### Variable Item Sizes

```rust
VirtualList::new(
    scroll_handle,
    items.len(),
    |ix, window, cx| {
        // Different heights based on content
        let height = if items[ix].len() > 50 {
            px(80.)  // Tall items for long content
        } else {
            px(40.)  // Normal height
        };
        size(px(300.), height)
    },
    |ix, bounds, selected, window, cx| {
        // Render logic
    },
)
```

## Theme Customization

### Scrollbar Appearance

Customize scrollbar appearance through theme configuration:

```rust
// In your theme JSON
{
    "scrollbar.background": "#ffffff20",
    "scrollbar.thumb.background": "#00000060",
    "scrollbar.thumb.hover.background": "#000000"
}
```

### Scrollbar Show Modes

Control when scrollbars are visible:

```rust
use gpui_component::scroll::ScrollbarShow;

// In theme initialization
theme.scrollbar_show = ScrollbarShow::Scrolling;  // Show only when scrolling
theme.scrollbar_show = ScrollbarShow::Hover;      // Show on hover
theme.scrollbar_show = ScrollbarShow::Always;     // Always visible
```

### System Integration

Sync scrollbar behavior with system preferences:

```rust
// Automatically sync with system settings
Theme::sync_scrollbar_appearance(cx);
```

## Advanced Usage

### ScrollableMask for Custom Scroll Areas

For advanced scroll control over specific areas:

```rust
use gpui_component::scroll::ScrollableMask;

ScrollableMask::new(Axis::Vertical, &scroll_handle)
    .debug() // Show debug borders
```

### Performance Optimization

For high-performance scrolling with many elements:

```rust
// Limit scroll update frequency
Scrollbar::vertical(&state, &handle)
    .max_fps(60) // Limit to 60 FPS during drag
```

### Nested Scrollable Areas

```rust
v_flex()
    .size_full()
    .child(
        // Outer vertical scroll
        v_flex()
            .flex_1()
            .scrollable(Axis::Vertical)
            .child(
                // Inner horizontal scroll
                h_flex()
                    .w_full()
                    .scrollable(Axis::Horizontal)
                    .child("Nested scrollable content")
            )
    )
```

## API Reference

### Scrollable

| Method               | Description                   |
| -------------------- | ----------------------------- |
| `new(axis, element)` | Create scrollable wrapper     |
| `vertical()`         | Set vertical scrolling only   |
| `horizontal()`       | Set horizontal scrolling only |
| `set_axis(axis)`     | Change scroll axis            |

### ScrollbarAxis

| Variant      | Description                  |
| ------------ | ---------------------------- |
| `Vertical`   | Vertical scrollbar only      |
| `Horizontal` | Horizontal scrollbar only    |
| `Both`       | Both vertical and horizontal |

### ScrollbarState

| Method      | Description                |
| ----------- | -------------------------- |
| `default()` | Create new scrollbar state |

### Scrollbar

| Method                      | Description                 |
| --------------------------- | --------------------------- |
| `vertical(state, handle)`   | Create vertical scrollbar   |
| `horizontal(state, handle)` | Create horizontal scrollbar |
| `both(state, handle)`       | Create both scrollbars      |
| `axis(axis)`                | Set scrollbar axis          |
| `scroll_size(size)`         | Set custom content size     |

### VirtualListScrollHandle

| Method                            | Description               |
| --------------------------------- | ------------------------- |
| `new()`                           | Create new handle         |
| `scroll_to_item(index, strategy)` | Scroll to specific item   |
| `offset()`                        | Get current scroll offset |
| `set_offset(point)`               | Set scroll position       |
| `content_size()`                  | Get total content size    |

### ScrollStrategy

| Variant  | Description          |
| -------- | -------------------- |
| `Top`    | Align item to top    |
| `Center` | Center item in view  |
| `Bottom` | Align item to bottom |

### ScrollbarShow

| Variant     | Description             |
| ----------- | ----------------------- |
| `Scrolling` | Show only during scroll |
| `Hover`     | Show on hover           |
| `Always`    | Always visible          |

## Examples

### File Browser with Scrolling

```rust
pub struct FileBrowser {
    files: Vec<String>,
    scroll_state: ScrollbarState,
}

impl Render for FileBrowser {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .border_1()
            .border_color(cx.theme().border)
            .size_full()
            .child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .children(self.files.iter().map(|file| {
                        div()
                            .h(px(32.))
                            .w_full()
                            .px_2()
                            .flex()
                            .items_center()
                            .hover(|style| style.bg(cx.theme().secondary_hover))
                            .child(file.clone())
                    }))
                    .scrollable(Axis::Vertical)
            )
    }
}
```

### Chat Messages with Auto-scroll

```rust
pub struct ChatView {
    messages: Vec<String>,
    scroll_handle: ScrollHandle,
    should_auto_scroll: bool,
}

impl ChatView {
    fn add_message(&mut self, message: String) {
        self.messages.push(message);

        if self.should_auto_scroll {
            // Scroll to bottom for new messages
            let max_offset = self.scroll_handle.max_offset();
            self.scroll_handle.set_offset(point(px(0.), max_offset.y));
        }
    }
}
```

### Data Table with Virtual Scrolling

```rust
pub struct DataTable {
    data: Vec<Vec<String>>,
    scroll_handle: VirtualListScrollHandle,
}

impl Render for DataTable {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        VirtualList::new(
            self.scroll_handle.clone(),
            self.data.len(),
            |_ix, _window, _cx| size(px(800.), px(32.)), // Fixed row height
            |ix, bounds, _selected, _window, cx| {
                h_flex()
                    .size(bounds.size)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(self.data[ix].iter().map(|cell| {
                        div()
                            .flex_1()
                            .px_2()
                            .flex()
                            .items_center()
                            .child(cell.clone())
                    }))
                    .into_any_element()
            },
        )
    }
}
```

## Performance Tips

1. **Use Virtualization**: For lists with >100 items, use `VirtualList`
2. **Limit Scroll Updates**: Use `max_fps()` for heavy content during drag
3. **Optimize Render**: Avoid complex rendering in scroll event handlers
4. **Batch Updates**: Group multiple scroll changes together
5. **Memory Management**: Virtual lists only render visible items

## Best Practices

1. **Consistent Behavior**: Keep scroll behavior consistent across your app
2. **Visual Feedback**: Provide clear scroll indicators for long content
3. **Responsive Design**: Ensure scrolling works well on different screen sizes
4. **Error Handling**: Handle edge cases like empty content gracefully
5. **Testing**: Test with various content sizes and screen readers
