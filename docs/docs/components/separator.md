---
order: 20
title: Separator
description: Visually or semantically separates content.
---

# Separator

A simple horizontal or vertical divider to separate content.

## Usage

```sh
use gpui_component::separator::{Separator, Orientation};

// Horizontal separator (default)
Separator::horizontal()

// Vertical separator
Separator::vertical()
```

## Examples

### Horizontal Separator

```sh
use gpui_component::prelude::*;
use gpui_component::separator::Separator;

div()
    .child("Above")
    .child(Separator::horizontal())
    .child("Below")
```

### Vertical Separator

```sh
div()
    .flex()
    .flex_row()
    .child("Left")
    .child(Separator::vertical())
    .child("Right")
```

## API Reference

- **`Separator::horizontal()`** – Creates a full‑width horizontal line.
- **`Separator::vertical()`** – Creates a full‑height vertical line.

Both instances implement `Styled`, so you can further customize them with methods like `.w()`, `.h()`, `.bg()`, etc.
