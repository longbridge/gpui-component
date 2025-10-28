---
title: Indicator
description: Displays an animated loading indicator showing the completion progress of a task.
---

# Indicator

A versatile indicator component that displays an animated loading spinner. Perfect for showing loading states, progress indicators, and other visual feedback during asynchronous operations. Features customizable icons, colors, sizes, and rotation animations.

## Import

```rust
use gpui_component::indicator::Indicator;
```

## Usage

### Basic Indicator

```rust
// Default loader icon
Indicator::new()
```

### Indicator with Custom Color

```rust
use gpui_component::ActiveTheme;

// Blue indicator
Indicator::new()
    .color(cx.theme().blue)

// Green indicator for success states
Indicator::new()
    .color(cx.theme().green)

// Custom color
Indicator::new()
    .color(cx.theme().cyan)
```

### Indicator Sizes

```rust
// Extra small indicator
Indicator::new().xsmall()

// Small indicator
Indicator::new().small()

// Medium indicator (default)
Indicator::new()

// Large indicator
Indicator::new().large()

// Custom size
Indicator::new().with_size(px(64.))
```

### Indicator with Custom Icon

```rust
use gpui_component::IconName;

// Loading circle icon
Indicator::new()
    .icon(IconName::LoaderCircle)

// Large loading circle with custom color
Indicator::new()
    .icon(IconName::LoaderCircle)
    .large()
    .color(cx.theme().cyan)

// Different loading icons
Indicator::new()
    .icon(IconName::Loader)
    .color(cx.theme().primary)
```

## Available Icons

The Indicator component supports various loading and progress icons:

### Loading Icons

- `Loader` (default) - Rotating line spinner
- `LoaderCircle` - Circular loading indicator

### Other Compatible Icons

- Any icon from the `IconName` enum can be used, though loading-specific icons work best with the rotation animation

## Animation

The Indicator component features a built-in rotation animation:

- **Duration**: 0.8 seconds (configurable via speed parameter)
- **Easing**: Ease-in-out transition
- **Repeat**: Infinite loop
- **Transform**: 360-degree rotation

## Size Reference

| Size        | Method              | Approximate Pixels |
| ----------- | ------------------- | ------------------ |
| Extra Small | `.xsmall()`         | ~12px              |
| Small       | `.small()`          | ~14px              |
| Medium      | (default)           | ~16px              |
| Large       | `.large()`          | ~24px              |
| Custom      | `.with_size(px(n))` | n px               |

## API Reference

### Indicator

| Method         | Description                                     |
| -------------- | ----------------------------------------------- |
| `new()`        | Create a new indicator with default loader icon |
| `icon(icon)`   | Set custom icon (accepts `IconName` or `Icon`)  |
| `color(color)` | Set indicator color (accepts `Hsla`)            |

### Size Methods (from Sizable trait)

| Method            | Description                     |
| ----------------- | ------------------------------- |
| `xsmall()`        | Extra small indicator size      |
| `small()`         | Small indicator size            |
| `medium()`        | Medium indicator size (default) |
| `large()`         | Large indicator size            |
| `with_size(size)` | Custom size in pixels           |

## Examples

### Loading States

```rust
// Simple loading spinner
Indicator::new()

// Loading with custom color
Indicator::new()
    .color(cx.theme().blue)

// Large loading indicator
Indicator::new()
    .large()
    .color(cx.theme().primary)
```

### Different Loading Icons

```rust
// Default loader (line spinner)
Indicator::new()
    .color(cx.theme().muted_foreground)

// Circle loader
Indicator::new()
    .icon(IconName::LoaderCircle)
    .color(cx.theme().blue)

// Large circle loader with custom color
Indicator::new()
    .icon(IconName::LoaderCircle)
    .large()
    .color(cx.theme().green)
```

### Status Indicators

```rust
// Loading state
Indicator::new()
    .small()
    .color(cx.theme().muted_foreground)

// Processing state
Indicator::new()
    .icon(IconName::LoaderCircle)
    .color(cx.theme().blue)

// Success processing (still animating)
Indicator::new()
    .icon(IconName::LoaderCircle)
    .color(cx.theme().green)
```

### Size Variations

```rust
// Extra small for inline text
Indicator::new()
    .xsmall()
    .color(cx.theme().muted_foreground)

// Small for buttons
Indicator::new()
    .small()
    .color(cx.theme().primary_foreground)

// Medium for general use (default)
Indicator::new()
    .color(cx.theme().primary)

// Large for prominent loading states
Indicator::new()
    .large()
    .color(cx.theme().blue)

// Custom size for specific requirements
Indicator::new()
    .with_size(px(32.))
    .color(cx.theme().orange)
```

### In UI Components

```rust
// In a button
Button::new("submit-btn")
    .loading(true)
    .icon(
        Indicator::new()
            .small()
            .color(cx.theme().primary_foreground)
    )
    .label("Loading...")

// In a card header
div()
    .flex()
    .items_center()
    .gap_2()
    .child("Processing...")
    .child(
        Indicator::new()
            .small()
            .color(cx.theme().muted_foreground)
    )

// Full-screen loading
div()
    .flex()
    .items_center()
    .justify_center()
    .h_full()
    .w_full()
    .child(
        Indicator::new()
            .large()
            .color(cx.theme().primary)
    )
```

## Performance Considerations

- The animation uses CSS transforms for optimal performance
- Multiple indicators on the same page share the same animation timing
- The component is lightweight and suitable for frequent updates
- Consider using smaller sizes for better performance with many indicators

## Common Patterns

### Conditional Loading

```rust
// Show indicator only when loading
.when(is_loading, |this| {
    this.child(
        Indicator::new()
            .small()
            .color(cx.theme().muted_foreground)
    )
})
```

### Loading with Text

```rust
// Loading text with indicator
h_flex()
    .items_center()
    .gap_2()
    .child(
        Indicator::new()
            .small()
            .color(cx.theme().primary)
    )
    .child("Loading data...")
```

### Overlay Loading

```rust
// Full overlay with indicator
div()
    .absolute()
    .inset_0()
    .flex()
    .items_center()
    .justify_center()
    .bg(cx.theme().background.alpha(0.8))
    .child(
        v_flex()
            .items_center()
            .gap_3()
            .child(
                Indicator::new()
                    .large()
                    .color(cx.theme().primary)
            )
            .child("Loading...")
    )
```
