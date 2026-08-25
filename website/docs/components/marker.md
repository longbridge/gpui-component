---
title: Marker
description: A compact composable row for conversation status, notifications, and separators.
---

# Marker

`Marker` is a lightweight conversation row for short status labels, notification boundaries, and timeline separators. It accepts arbitrary children instead of defining application-specific status data.

## Import

```rust
use gpui_component::marker::{
    Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant,
};
use gpui_component::shimmer::{ShimmerStyle, ShimmerText};
```

## Status marker

Compose an existing icon, spinner, badge, or text value directly:

```rust
Marker::new()
    .text_color(cx.theme().green)
    .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
    .content(MarkerContent::new().child("Online"))

Marker::new()
    .icon(MarkerIcon::new().child(Spinner::new().xsmall()))
    .content(MarkerContent::new().child("Alice is typing…"))
```

`MarkerIcon` supplies the standard compact icon slot, while `MarkerContent` gives text or rich content its own style target. Their spacing and typography follow the application's `rem` scale. Direct children remain supported for application-specific layouts. The component intentionally has no `Online`, `Typing`, `Read`, or other status enum. Those meanings and colors belong to the application.

## Loading styles

Enable loading without changing the marker variant, layout, or normal appearance:

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Spinner)
    .content(MarkerContent::new().text("Loading messages…"))

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .content(MarkerContent::new().text("Thinking…"))
```

`Spinner` is the default loading style and adds a compact spinner when no `MarkerIcon` was supplied. An explicitly composed icon always takes precedence.

`Shimmer` moves a continuous, theme-aware highlight across text added with `MarkerContent::text(...)`, matching the text-only thinking treatment used by ChatGPT. The highlight follows the inherited text color and semantic theme colors, staying visually bright in both light and dark themes. Existing arbitrary `MarkerContent` children remain supported through a gentle opacity pulse. Icons and separator lines stay static, and enabling reduced motion displays clear, non-animated text.

The animation respects inherited and explicitly customized text colors. Both loading styles work with every `MarkerVariant`, and loading is disabled by default.

### Configure the shimmer

Use a reusable `ShimmerStyle` to adjust the sweep without replacing the Marker composition:

```rust
use std::time::Duration;

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .highlight_color(cx.theme().primary)
            .spread(0.45)
            .reverse(true),
    )
    .content(MarkerContent::new().text("Processing…"))
```

The default sweep takes two seconds, uses a theme-aware highlight, and has a normalized spread of `0.3`. `spread(...)` accepts the text-relative `0.05..=1.0` range. `reverse(true)` moves the highlight from right to left.

### Reuse shimmer anywhere

`ShimmerText` provides the same effect without requiring a Marker:

```rust
ShimmerText::new("Uploading report.pdf…")
    .with_shimmer_style(ShimmerStyle::new().spread(0.4))
    .text_sm()
    .text_color(cx.theme().muted_foreground)
```

Its direct `duration(...)`, `highlight_color(...)`, `spread(...)`, and `reverse(...)` builders provide the same adjustments. `ShimmerText` implements `Styled`, inherits surrounding typography and text color, preserves wrapping and truncation, and disables animation automatically when reduced motion is enabled.

## Variants

### Plain

```rust
Marker::new()
    .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
    .content(MarkerContent::new().child("Conversation archived"))
```

### Separator

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .content(MarkerContent::new().child("Today"))
```

The decorative lines remain internal because they carry no semantic content. Use `separator_style(...)` to refine their color, thickness, or spacing without introducing public line subcomponents.

### Border

```rust
Marker::new()
    .with_variant(MarkerVariant::Border)
    .content(MarkerContent::new().child("3 unread messages"))
```

## Custom styling

`Marker`, `MarkerIcon`, and `MarkerContent` implement `Styled`, and refinements are applied after their defaults:

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
- [MarkerLoadingStyle]
- [MarkerIcon]
- [MarkerContent]
- [ShimmerStyle]
- [ShimmerText]

[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[MarkerLoadingStyle]: https://docs.rs/gpui-component/latest/gpui_component/marker/enum.MarkerLoadingStyle.html
[MarkerIcon]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerIcon.html
[MarkerContent]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerContent.html
[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
[ShimmerText]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerText.html
