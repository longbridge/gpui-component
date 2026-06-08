---
title: StatusBar
description: A horizontal status bar with left, center, and right regions, usually placed at the bottom of a window or pane.
---

# StatusBar

StatusBar is a horizontal bar split into three regions — `left`, `center`, and `right`. It is usually placed at the bottom of a window or pane to show contextual information and quick actions.

The design mirrors the status bars found in native UI frameworks: Windows `StatusStrip`, WPF `StatusBar`, and macOS `NSStatusBar`.

## Import

```rust
use gpui_component::status_bar::StatusBar;
```

## Regions

Pass any `impl IntoElement` — a string, an `Icon`, a `Button`, a custom layout, etc. — to a region. `left` and `right` pin items to each end; `child` / `children` add to the center, whose alignment follows the pinned ends — centered with both `left` and `right`, end-aligned with only `left`, start-aligned otherwise (only `right`, or neither, like a plain container). Call a method multiple times to add more.

- For a **non-interactive label**, pass a plain string — it inherits the bar's text style and has no hover.
- For a **clickable button**, use `StatusBar::button(id)`, which returns a ghost, xsmall `Button` so every status bar button stays a consistent size. Chain `label`, `icon`, `tooltip`, `on_click`, etc.
- For a **separator**, use `StatusBar::separator()`.
- For anything else, pass the element directly.

## Usage

### Labels

```rust
StatusBar::new()
    .left("Ready")
    .child("README.md")
    .right("UTF-8")
```

### Buttons

```rust
StatusBar::new()
    .left(
        StatusBar::button("branch")
            .icon(IconName::Github)
            .label("main")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .right(
        StatusBar::button("go-to-line")
            .label("Ln 1, Col 1")
            .tooltip("Go to Line/Column")
            .on_click(cx.listener(|this, _, window, cx| { /* ... */ })),
    )
```

### Separators and custom elements

```rust
StatusBar::new()
    .left(StatusBar::button("branch").icon(IconName::Github).label("main"))
    .left(StatusBar::separator())
    .left(
        // Any custom element works.
        h_flex()
            .items_center()
            .gap_1()
            .child(Icon::new(IconName::CircleCheck).xsmall())
            .child("0 problems"),
    )
    .child(Progress::new("indexing").value(60.).w_24())
```

### Custom styling

`StatusBar` implements `Styled`, so any style method overrides the defaults.

```rust
StatusBar::new()
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .left("Ready")
```

## API Reference

### StatusBar

| Method            | Description                                          |
| ----------------- | ---------------------------------------------------- |
| `new()`           | Create a new, empty status bar                       |
| `button(id)`      | A ghost, xsmall `Button` preset for the status bar   |
| `separator()`     | A vertical separator for splitting items into groups |
| `left(child)`     | Append an element to the left region (call to add more) |
| `right(child)`    | Append an element to the right region                |
| `child(c)` / `children(cs)` | Add element(s) to the center region        |

Each region method takes `impl IntoElement`. `StatusBar` also implements `Styled`, so style methods (`bg`, `border_color`, `py`, etc.) can override the defaults.

## Notes

- The center (via `child` / `children`) is centered with both `left` and `right`, end-aligned with only `left`, and start-aligned otherwise (only `right`, or neither — like a plain container).
- Use a plain string (or any non-interactive element) for read-only items to avoid the button hover effect; use `StatusBar::button` only for clickable items.
