---
title: Badge
description: A red dot that indicates the number of unread messages, status, or other notifications.
---

# Badge

A versatile badge component that can display counts, dots, or icons on elements. Perfect for indicating notifications, status, or other contextual information on avatars, icons, or other UI elements.

## Import

```rust
use gpui_component::badge::Badge;
```

## Usage

### Basic Badge with Count

```rust
Badge::new()
    .count(3)
    .child(Icon::new(IconName::Bell))
```

### Badge Variants

```rust
// Number badge (default)
Badge::new()
    .count(5)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))

// Dot badge
Badge::new()
    .dot()
    .child(Icon::new(IconName::Inbox))

// Icon badge
Badge::new()
    .icon(IconName::Check)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))
```

### Badge Sizes

```rust
// Small badge
Badge::new()
    .small()
    .count(1)
    .child(Avatar::new().small())

// Medium badge (default)
Badge::new()
    .count(5)
    .child(Avatar::new())

// Large badge
Badge::new()
    .large()
    .count(10)
    .child(Avatar::new().large())
```

### Badge Colors

```rust
use gpui_component::ActiveTheme;

// Custom colors
Badge::new()
    .count(3)
    .color(cx.theme().blue)
    .child(Avatar::new())

Badge::new()
    .icon(IconName::Star)
    .color(cx.theme().yellow)
    .child(Avatar::new())

Badge::new()
    .dot()
    .color(cx.theme().green)
    .child(Icon::new(IconName::Bell))
```

### Badge on Icons

```rust
use gpui_component::{Icon, IconName};

// Badge with count on icon
Badge::new()
    .count(3)
    .child(Icon::new(IconName::Bell).large())

// Badge with high count (shows max)
Badge::new()
    .count(103)
    .child(Icon::new(IconName::Inbox).large())

// Custom max count
Badge::new()
    .count(150)
    .max(999)
    .child(Icon::new(IconName::Mail))
```

### Badge on Avatars

```rust
use gpui_component::avatar::Avatar;

// Basic count badge
Badge::new()
    .count(5)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))

// Status badge with icon
Badge::new()
    .icon(IconName::Check)
    .color(cx.theme().green)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))

// Online indicator with dot
Badge::new()
    .dot()
    .color(cx.theme().green)
    .child(Avatar::new().src("https://example.com/avatar.jpg"))
```

### Complex Nested Badges

```rust
// Badge on badge for complex status
Badge::new()
    .count(212)
    .large()
    .child(
        Badge::new()
            .icon(IconName::Check)
            .large()
            .color(cx.theme().cyan)
            .child(Avatar::new().large().src("https://example.com/avatar.jpg"))
    )

// Multiple status indicators
Badge::new()
    .count(2)
    .color(cx.theme().green)
    .large()
    .child(
        Badge::new()
            .icon(IconName::Star)
            .large()
            .color(cx.theme().yellow)
            .child(Avatar::new().large().src("https://example.com/avatar.jpg"))
    )
```

## API Reference

### Badge

| Method           | Description                                  |
| ---------------- | -------------------------------------------- |
| `new()`          | Create a new badge                           |
| `count(usize)`   | Set the count to display (0 hides the badge) |
| `dot()`          | Display as a small dot indicator             |
| `icon(icon)`     | Display an icon instead of count             |
| `max(usize)`     | Set maximum count to show (default: 99)      |
| `color(color)`   | Set badge background color                   |
| `child(element)` | Add child element to position badge on       |

### Size Methods (from Sizable trait)

| Method            | Description                       |
| ----------------- | --------------------------------- |
| `small()`         | Small badge size (10px)           |
| `medium()`        | Medium badge size (16px, default) |
| `large()`         | Large badge size (24px)           |
| `with_size(size)` | Custom size                       |

## Examples

### Notification Indicators

```rust
// Unread messages
Badge::new()
    .count(12)
    .child(Icon::new(IconName::Mail).large())

// New notifications
Badge::new()
    .count(3)
    .color(cx.theme().red)
    .child(Icon::new(IconName::Bell).large())

// High priority with custom max
Badge::new()
    .count(1234)
    .max(999)
    .color(cx.theme().orange)
    .child(Icon::new(IconName::AlertTriangle))
```

### Status Indicators

```rust
// Online status
Badge::new()
    .dot()
    .color(cx.theme().green)
    .child(Avatar::new().src("https://example.com/user.jpg"))

// Verified status
Badge::new()
    .icon(IconName::CheckCircle)
    .color(cx.theme().blue)
    .child(Avatar::new().src("https://example.com/verified-user.jpg"))

// Warning status
Badge::new()
    .icon(IconName::AlertTriangle)
    .color(cx.theme().yellow)
    .child(Avatar::new().src("https://example.com/user.jpg"))
```

### Different Badge Positions

```rust
// The badge automatically positions itself based on variant:
// - Dot: top-right corner (small dot)
// - Number: top-right with dynamic sizing
// - Icon: bottom-right corner with border
```

### Count Formatting

```rust
// Numbers 1-99 show as-is
Badge::new().count(5)    // Shows "5"
Badge::new().count(99)   // Shows "99"

// Numbers above max show with "+"
Badge::new().count(100)  // Shows "99+" (default max)
Badge::new().count(1000).max(999) // Shows "999+"

// Zero count hides the badge
Badge::new().count(0)    // Badge not visible
```

## Accessibility

- Badge content is announced by screen readers
- High contrast colors ensure visibility
- Icon badges include appropriate semantic meaning
- Badge positioning doesn't interfere with clickable areas
- Color is not the only indicator of status (icons and text provide additional context)

## Behavior Notes

- Badges with count of 0 are automatically hidden
- Dot and icon variants are always visible regardless of count
- Badge size automatically adjusts based on content length
- Multiple badges can be nested for complex status indicators
- Badge positioning is absolute and doesn't affect layout flow
