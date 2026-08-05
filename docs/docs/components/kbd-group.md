---
title: KbdGroup
description: Groups multiple keyboard shortcuts with a \"+\" separator.
---

# KbdGroup

A component for grouping multiple `Kbd` elements, automatically inserting a \"+\" separator between them. This mimics the visual style of key‑binding instructions like **Ctrl+C** or **Cmd+Shift+P**.

## Import

```sh
use gpui_component::kbd::Kbd;
use gpui_component::kbd_group::KbdGroup;
```

## Usage

### Basic Group

```sh
KbdGroup::new()
    .child(Kbd::new(Keystroke::parse("cmd-c").unwrap()))
    .child(Kbd::new(Keystroke::parse("ctrl-v").unwrap()))
```

### Multiple Keys

```sh
KbdGroup::new()
    .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
    .child(Kbd::new(Keystroke::parse("cmd-ctrl-t").unwrap()))
    .child(Kbd::new(Keystroke::parse("escape").unwrap()))
```

### Styling the Group Container

```sh
KbdGroup::new()
    .child(Kbd::new(Keystroke::parse("cmd-s").unwrap()))
    .child(Kbd::new(Keystroke::parse("cmd-enter").unwrap()))
    .gap_2()                // increase spacing
    .text_sm()              // adjust font size
```

## Platform Differences

Because each `Kbd` element already adapts to the platform, `KbdGroup` will automatically display the correct symbols or text labels and separator format.

## Examples

### Inline Shortcut Hint

```sh
h_flex()
    .gap_2()
    .items_center()
    .child("Save project (")
    .child(KbdGroup::new()
        .child(Kbd::new(Keystroke::parse("cmd-shift-s").unwrap()))
    )
    .child(")")
```

### Menu Item with Shortcut

```sh
h_flex()
    .justify_between()
    .items_center()
    .child("Find in Files")
    .child(KbdGroup::new()
        .child(Kbd::new(Keystroke::parse("cmd-shift-f").unwrap()))
    )
```

### Tooltip with Keyboard Combination

```sh
Button::new("undo")
    .label("Undo")
    .tooltip(
        KbdGroup::new()
            .child(Kbd::new(Keystroke::parse("cmd-z").unwrap()))
            .child(Kbd::new(Keystroke::parse("ctrl-z").unwrap()))
    )
```

## API Reference

- **`KbdGroup::new()`** – Creates an empty group.
- **`.child(kbd)`** – Adds a `Kbd` element to the group; a \"+\" separator is automatically inserted between consecutive children.
- The component implements `Styled`, so container‑level styling is fully customisable.

## Styling

The `KbdGroup` inherits its base appearance from the individual `Kbd` components. The container itself is a horizontal flex layout with a small gap. You can apply additional styling via the `Styled` trait methods.
