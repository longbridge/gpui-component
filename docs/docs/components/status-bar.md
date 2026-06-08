---
title: StatusBar
description: A horizontal status bar with left, center, and right regions, usually placed at the bottom of a window or pane.
---

# StatusBar

StatusBar is a horizontal bar split into three regions — `left`, `center`, and `right` — distributed with `justify_between`. It is usually placed at the bottom of a window or pane to show contextual information and quick actions.

The design mirrors the status bars found in native UI frameworks: Windows `StatusStrip`, WPF `StatusBar`, and macOS `NSStatusBar`. Each region accepts any element, and the companion `StatusBarItem` covers the common icon + label + click pattern.

## Import

```rust
use gpui_component::status_bar::{StatusBar, StatusBarItem};
```

## Usage

### Left and Right

```rust
StatusBar::new()
    .left(StatusBarItem::new("status").label("Ready"))
    .right(StatusBarItem::new("encoding").label("UTF-8"))
```

### Three Regions

Call `left`, `center`, or `right` multiple times to add more items to a region.

```rust
StatusBar::new()
    .left(StatusBarItem::new("branch").icon(IconName::Github).label("main"))
    .center(StatusBarItem::new("title").label("README.md"))
    .right(StatusBarItem::new("position").label("Ln 1, Col 1"))
```

### Items with Icons

```rust
StatusBar::new()
    .left(StatusBarItem::new("info").icon(IconName::Info).label("12 issues"))
    .right(StatusBarItem::new("lang").icon(IconName::Globe).label("Rust"))
```

### Clickable Items

Setting `on_click` makes an item trigger the handler when clicked.

```rust
StatusBar::new()
    .left(
        StatusBarItem::new("go-to-line")
            .icon(IconName::CircleCheck)
            .label("Ln 1, Col 1")
            .tooltip("Go to Line/Column")
            .on_click(cx.listener(|this, _, window, cx| {
                // handle click
            })),
    )
    .right(StatusBarItem::new("encoding").label("UTF-8"))
```

### Custom Styling

`StatusBar` implements `Styled`, so any style method overrides the defaults.

```rust
StatusBar::new()
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .left(StatusBarItem::new("status").label("Ready"))
```

## API Reference

### StatusBar

| Method           | Description                                            |
| ---------------- | ----------------------------------------------------- |
| `new()`          | Create a new, empty status bar                        |
| `left(child)`    | Append a child to the left region (call to add more)  |
| `center(child)`  | Append a child to the center region                   |
| `right(child)`   | Append a child to the right region                    |

`StatusBar` also implements `Styled`, so style methods (`bg`, `border_color`, `py`, etc.) can override the defaults.

### StatusBarItem

A `StatusBarItem` renders as a ghost `xsmall` `Button`, so it matches the size and styling of buttons placed in the same status bar.

| Method            | Description                          |
| ----------------- | ------------------------------------ |
| `new(id)`         | Create a new item with the given id  |
| `icon(icon)`      | Set the leading icon                 |
| `label(text)`     | Set the label text                   |
| `tooltip(text)`   | Set the tooltip shown on hover       |
| `on_click(fn)`    | Set the click handler                |

## Notes

- The three regions are distributed with `justify_between`; an empty `center` keeps `left` and `right` pinned to each end.
- `StatusBar` sets `flex_shrink_0`, so it keeps its height when placed at the bottom of a flex column next to a `flex_1` content area.
- Regions accept any element, so you can place `Button`, `Progress`, or other components alongside `StatusBarItem`.
