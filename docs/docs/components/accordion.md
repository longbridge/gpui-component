---
title: Accordion
description: The accordion uses collapse internally to make it collapsible.
---

# Accordion

An accordion component that allows users to show and hide sections of content. It uses collapse functionality internally to create collapsible panels.

## Import

```rust
use gpui_component::accordion::Accordion;
```

## Usage

### Basic Accordion

```rust
Accordion::new("my-accordion")
    .item(|item| {
        item.title("Section 1")
            .content("Content for section 1")
    })
    .item(|item| {
        item.title("Section 2")
            .content("Content for section 2")
    })
    .item(|item| {
        item.title("Section 3")
            .content("Content for section 3")
    })
```

### Multiple Open Items

By default, only one accordion item can be open at a time. Use `multiple()` to allow multiple items to be open:

```rust
Accordion::new("my-accordion")
    .multiple(true)
    .item(|item| item.title("Section 1").content("Content 1"))
    .item(|item| item.title("Section 2").content("Content 2"))
```

### With Borders

```rust
Accordion::new("my-accordion")
    .bordered(true)
    .item(|item| item.title("Section 1").content("Content 1"))
```

### Different Sizes

```rust
use gpui_component::{Sizable as _, Size};

Accordion::new("my-accordion")
    .small()
    .item(|item| item.title("Small Section").content("Content"))

Accordion::new("my-accordion")
    .large()
    .item(|item| item.title("Large Section").content("Content"))
```

### Handle Toggle Events

```rust
Accordion::new("my-accordion")
    .on_toggle_click(|open_indices, window, cx| {
        println!("Open items: {:?}", open_indices);
    })
    .item(|item| item.title("Section 1").content("Content 1"))
```

### Disabled State

```rust
Accordion::new("my-accordion")
    .disabled(true)
    .item(|item| item.title("Disabled Section").content("Content"))
```

## API Reference

- [Accordion]
- [AccordionItem]

### Sizing

Implements [Sizable] trait:

- `small()` - Small size
- `medium()` - Medium size (default)
- `large()` - Large size
- `xsmall()` - Extra small size

## Examples

### With Custom Icons

```rust
Accordion::new("my-accordion")
    .item(|item| {
        item.title(
            h_flex()
                .gap_2()
                .child(Icon::new(IconName::Settings))
                .child("Settings")
        )
        .content("Settings content here")
    })
```

### Nested Accordions

```rust
Accordion::new("outer")
    .item(|item| {
        item.title("Parent Section")
            .content(
                Accordion::new("inner")
                    .item(|item| item.title("Child 1").content("Content"))
                    .item(|item| item.title("Child 2").content("Content"))
            )
    })
```

[Accordion]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.Accordion.html
[AccordionItem]: https://docs.rs/gpui-component/latest/gpui_component/accordion/struct.AccordionItem.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
