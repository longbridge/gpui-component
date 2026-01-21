---
name: gpui-layout-and-style
description: Layout and styling in GPUI. Use when styling components, layout systems, or CSS-like properties.
---

## Overview

GPUI provides CSS-like styling with Rust type safety.

**Key Concepts:**
- Flexbox layout system
- Styled trait for chaining styles
- Size units: `px()`, `rems()`, `relative()`
- Colors, borders, shadows

## Quick Start

### Basic Styling

```rust
use gpui::*;

div()
    .w(px(200.))
    .h(px(100.))
    .bg(rgb(0x2196F3))
    .text_color(rgb(0xFFFFFF))
    .rounded(px(8.))
    .p(px(16.))
    .child("Styled content")
```

### Flexbox Layout

```rust
div()
    .flex()
    .flex_row()  // or flex_col() for column
    .gap(px(8.))
    .items_center()
    .justify_between()
    .children([
        div().child("Item 1"),
        div().child("Item 2"),
        div().child("Item 3"),
    ])
```

### Size Units

```rust
div()
    .w(px(200.))           // Pixels
    .h(rems(10.))          // Relative to font size
    .w(relative(0.5))      // 50% of parent
    .min_w(px(100.))
    .max_w(px(400.))
```

## Common Patterns

### Centered Content

```rust
div()
    .flex()
    .items_center()
    .justify_center()
    .size_full()
    .child("Centered")
```

### Card Layout

```rust
div()
    .w(px(300.))
    .bg(cx.theme().surface)
    .rounded(px(8.))
    .shadow_md()
    .p(px(16.))
    .gap(px(12.))
    .flex()
    .flex_col()
    .child(heading())
    .child(content())
```

### Responsive Spacing

```rust
div()
    .p(px(16.))           // Padding all sides
    .px(px(20.))          // Padding horizontal
    .py(px(12.))          // Padding vertical
    .pt(px(8.))           // Padding top
    .gap(px(8.))          // Gap between children
```

## Styling Methods

### Dimensions

```rust
.w(px(200.))              // Width
.h(px(100.))              // Height
.size(px(200.))           // Width and height
.min_w(px(100.))          // Min width
.max_w(px(400.))          // Max width
```

### Colors

```rust
.bg(rgb(0x2196F3))        // Background
.text_color(rgb(0xFFFFFF)) // Text color
.border_color(rgb(0x000000)) // Border color
```

### Borders

```rust
.border(px(1.))           // Border width
.rounded(px(8.))          // Border radius
.rounded_t(px(8.))        // Top corners
.border_color(rgb(0x000000))
```

### Spacing

```rust
.p(px(16.))               // Padding
.m(px(8.))                // Margin
.gap(px(8.))              // Gap between flex children
```

### Flexbox

```rust
.flex()                   // Enable flexbox
.flex_row()               // Row direction
.flex_col()               // Column direction
.items_center()           // Align items center
.justify_between()        // Space between items
.flex_grow()              // Grow to fill space
```

## Theme Integration

```rust
div()
    .bg(cx.theme().surface)
    .text_color(cx.theme().foreground)
    .border_color(cx.theme().border)
    .when(is_hovered, |el| {
        el.bg(cx.theme().hover)
    })
```

## Conditional Styling

```rust
div()
    .when(is_active, |el| {
        el.bg(cx.theme().primary)
    })
    .when_some(optional_color, |el, color| {
        el.bg(color)
    })
```

## Reference Documentation

- **Complete Guide**: See [reference.md](references/reference.md)
  - All styling methods
  - Layout strategies
  - Theming, responsive design
