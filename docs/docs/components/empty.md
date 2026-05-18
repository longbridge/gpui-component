---
title: Empty
description: Placeholder for empty states.
---

# Empty

Use the `Empty` component to fill a space when there is no content to display. It can show an icon, a title, a description, and an optional action button.

## Import

```sh
use gpui_component::empty::Empty;
```

## Usage

### Basic Empty State

```sh
Empty::new()
    .title("No items")
    .description("Your list is empty.")
```

### Full Example

```sh
Empty::new()
    .icon(IconName::Inbox)
    .title("Inbox zero!")
    .description("You have no new messages.")
    .action(
        Button::new("refresh")
            .primary()
            .label("Refresh"),
    )
```

## API Reference

- **`Empty::new()`** – Creates an empty container.
- **`.icon(…)`** – Sets an icon element.
- **`.title(…)`** – Sets the title text.
- **`.description(…)`** – Sets a short description.
- **`.action(…)`** – Adds a button.

The component implements `Styled`, so you can tailor the container with any standard style methods.

## Styling

All default styles are minimal. You can override the layout and spacing via `Styled` to match your design system.
