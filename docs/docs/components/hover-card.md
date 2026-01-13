---
title: HoverCard
description: A floating overlay that displays rich content when hovering over a trigger element.
---

# HoverCard

HoverCard component for displaying rich content that appears when the mouse hovers over a trigger element. Ideal for previewing user profiles, link previews, and other contextual information without requiring a click. Features configurable delays for both opening and closing to prevent flickering during quick mouse movements.

## Import

```rust
use gpui_component::hover_card::HoverCard;
```

## Usage

### Basic HoverCard

```rust
use gpui::ParentElement as _;
use gpui_component::hover_card::HoverCard;

HoverCard::new("basic")
    .trigger(
        div()
            .child("Hover over me")
            .cursor_pointer()
    )
    .child("This content appears on hover")
```

### User Profile Preview

A common use case is showing user profiles when hovering over a username, similar to GitHub or Twitter:

```rust
use gpui_component::{
    avatar::Avatar,
    hover_card::HoverCard,
    Anchor,
};

HoverCard::new("user-profile")
    .anchor(Anchor::BottomCenter)
    .trigger(
        div()
            .child("@username")
            .text_color(cx.theme().primary)
            .cursor_pointer()
    )
    .child(
        h_flex()
            .gap_4()
            .child(Avatar::new().with_size(px(48.)))
            .child(
                v_flex()
                    .gap_1()
                    .child(div().child("Display Name").font_semibold())
                    .child(div().child("@username").text_xs())
                    .child(div().child("User bio goes here..."))
            )
    )
```

### Link Preview

Display a preview of a link's content when hovering over it:

```rust
HoverCard::new("link-preview")
    .anchor(Anchor::BottomCenter)
    .open_delay(500)  // Open faster for links
    .close_delay(200)
    .trigger(
        div()
            .child("example.com")
            .text_color(cx.theme().link)
            .underline()
            .cursor_pointer()
    )
    .child(
        v_flex()
            .gap_2()
            .w(px(300.))
            .child(div().child("Page Title").font_bold())
            .child(div().child("Page description...").text_sm())
    )
```

### Custom Timing

Adjust the opening and closing delays to suit your needs:

```rust
HoverCard::new("custom-timing")
    .open_delay(200)   // Open after 200ms (default is 700ms)
    .close_delay(100)  // Close after 100ms (default is 300ms)
    .trigger(Button::new("fast").label("Fast Response"))
    .child("This opens and closes quickly")
```

### Positioning

HoverCard supports 6 positioning options using the `Anchor` type:

```rust
use gpui_component::Anchor;

// Top positions
HoverCard::new("top-left").anchor(Anchor::TopLeft)
HoverCard::new("top-center").anchor(Anchor::TopCenter)
HoverCard::new("top-right").anchor(Anchor::TopRight)

// Bottom positions (default)
HoverCard::new("bottom-left").anchor(Anchor::BottomLeft)
HoverCard::new("bottom-center").anchor(Anchor::BottomCenter)
HoverCard::new("bottom-right").anchor(Anchor::BottomRight)
```

### Controlled Mode

You can control the open state externally:

```rust
struct MyView {
    card_open: bool,
}

HoverCard::new("controlled")
    .open(self.card_open)
    .on_open_change(cx.listener(|this, open, _, cx| {
        this.card_open = *open;
        println!("Card is now: {}", if *open { "open" } else { "closed" });
        cx.notify();
    }))
    .trigger(Button::new("btn").label("Hover"))
    .child("Controlled content")
```

### Custom Content Builder

For more complex content that needs access to the HoverCard state:

```rust
HoverCard::new("complex")
    .trigger(Button::new("btn").label("Hover me"))
    .content(|state, window, cx| {
        v_flex()
            .child("Dynamic content")
            .child(format!("Open: {}", state.is_open()))
    })
```

### Styling

HoverCard inherits all `Styled` trait methods:

```rust
HoverCard::new("styled")
    .trigger(Button::new("btn").label("Styled"))
    .w(px(400.))
    .max_h(px(500.))
    .text_sm()
    .gap_2()
    .child("Styled content")
```

Disable default appearance and apply custom styles:

```rust
HoverCard::new("custom-styled")
    .appearance(false)  // Disable default popover styling
    .trigger(Button::new("btn").label("Custom"))
    .bg(cx.theme().background)
    .border_2()
    .border_color(cx.theme().primary)
    .rounded(px(12.))
    .p_4()
    .child("Custom styled content")
```

## API Reference

### HoverCard Methods

- `new(id: impl Into<ElementId>)` - Create a new HoverCard with a unique ID
- `trigger<T: IntoElement>(trigger: T)` - Set the element that triggers the hover
- `content<F>(content: F)` - Set a content builder function
- `open_delay(ms: u64)` - Set delay before showing (default: 700ms)
- `close_delay(ms: u64)` - Set delay before hiding (default: 300ms)
- `anchor(anchor: impl Into<Anchor>)` - Set positioning (default: TopLeft)
- `open(open: bool)` - Force open state (controlled mode)
- `on_open_change<F>(callback: F)` - Callback when open state changes
- `appearance(appearance: bool)` - Enable/disable default styling (default: true)

### HoverCardState Methods

- `is_open() -> bool` - Check if the hover card is currently open

## Behavior Details

### Hover Timing

The HoverCard uses a sophisticated timing system to provide a smooth user experience:

1. **Open Delay (700ms default)**: Prevents the card from flickering when the mouse quickly passes over the trigger
2. **Close Delay (300ms default)**: Gives users time to move their mouse from the trigger to the content area without the card closing
3. **Interactive Content**: Users can move their mouse into the content area, and the card will remain open as long as the mouse is either on the trigger or in the content

### Edge Cases Handled

- **Quick Mouse Sweep**: If the mouse quickly moves across the trigger, the card won't open (cancelled by the open delay)
- **Trigger to Content Movement**: The card stays open when moving the mouse from the trigger to the content area
- **Rapid Hovering**: Multiple rapid hover events are debounced using an epoch-based timer system
- **Multiple HoverCards**: Each HoverCard has independent state, so multiple cards can coexist without interfering

## Best Practices

1. **Use appropriate delays**:
   - Standard content: 700ms open, 300ms close
   - Quick previews: 500ms open, 200ms close
   - Tooltips: 300ms open, 100ms close

2. **Keep content concise**: HoverCards should provide preview information, not full content

3. **Make triggers visually distinct**: Use colors, underlines, or cursor changes to indicate hoverable elements

4. **Consider accessibility**: HoverCards are visual-only and don't support keyboard navigation. For keyboard-accessible content, consider using a Popover instead

5. **Avoid nested HoverCards**: They can create confusing user experiences

## Differences from Popover

| Feature | HoverCard | Popover |
|---------|-----------|---------|
| Trigger | Mouse hover | Click/right-click |
| Keyboard navigation | No | Yes (with focus) |
| Dismiss on outside click | No | Yes (configurable) |
| Timing delays | Yes (open/close) | No |
| Primary use case | Previews | Actions/forms |

## Examples

See the [HoverCard Story](../../story) for interactive examples demonstrating:
- Basic hover cards
- User profile previews
- Link previews
- Custom timing configurations
- All positioning options
- Controlled mode

## Related Components

- [Popover](./popover.md) - Click-triggered overlay for actions and forms
- [Tooltip](./tooltip.md) - Simple text hints on hover
- [Avatar](./avatar.md) - User profile images (often used in HoverCard content)
