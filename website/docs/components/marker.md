---
title: Marker
description: A compact composable row for conversation status, notifications, and separators.
---

# Marker

`Marker` is a lightweight conversation row for short status labels, notification boundaries, and timeline separators. It accepts arbitrary children instead of defining application-specific status data.

## Import

```rust
use gpui_component::marker::{Marker, MarkerVariant};
```

## Status marker

Compose an existing icon, spinner, badge, or text value directly:

```rust
Marker::new()
    .text_color(cx.theme().green)
    .child(Icon::new(IconName::CircleCheck))
    .child("Online")

Marker::new()
    .child(Spinner::new().xsmall())
    .child("Alice is typing…")
```

The component intentionally has no `Online`, `Typing`, `Read`, or other status enum. Those meanings and colors belong to the application.

## Variants

### Plain

```rust
Marker::new()
    .child(Icon::new(IconName::Info))
    .child("Conversation archived")
```

### Separator

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .child("Today")
```

The decorative lines remain internal because they carry no semantic content. Use `separator_style(...)` to refine their color, thickness, or spacing without introducing public line subcomponents.

### Border

```rust
Marker::new()
    .with_variant(MarkerVariant::Border)
    .child("3 unread messages")
```

## Custom styling

`Marker` implements `Styled`, and refinements are applied after its defaults:

```rust
Marker::new()
    .px_3()
    .py_2()
    .rounded(cx.theme().radius)
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .child(Icon::new(IconName::Star))
    .child("Pinned message")
```

Interactive children should use `Button` or `Link`; Marker itself remains a non-interactive semantic container.

For long or non-wrapping content, compose a styled `div().min_w_0()` child and choose the wrapping or truncation behavior at the call site. Marker does not impose text overflow rules on arbitrary children.

## When Marker is unnecessary

Marker is a convenience component and deliberately stays thin. Prefer an existing component when it already expresses the entire task:

- `Badge` for only a count or dot.
- `Tag` for a standalone labeled status.
- `Separator::horizontal().label(...)` for only a labeled divider.
- `h_flex()` for an application-specific icon and text row with no shared marker styling.

Use Marker when those contents need one consistent conversation-row surface or must switch between plain, separator, and border treatments.

## API reference

- [Marker]

[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
