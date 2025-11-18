---
title: Button
description: Displays a button or a component that looks like a button.
---

# Button

The [Button] element with multiple variants, sizes, and states. Supports icons, loading states, and can be grouped together.

## Import

```rust
use gpui_component::button::{Button, ButtonGroup};
```

## Usage

### Basic Button

```rust
Button::new("my-button")
    .label("Click me")
    .on_click(|_, _, _| {
        println!("Button clicked!");
    })
```

### Variants

```rust
// Primary button
Button::new("btn-primary").primary().label("Primary")

// Secondary button (default)
Button::new("btn-secondary").label("Secondary")

// Danger button
Button::new("btn-danger").danger().label("Delete")

// Warning button
Button::new("btn-warning").warning().label("Warning")

// Success button
Button::new("btn-success").success().label("Success")

// Info button
Button::new("btn-info").info().label("Info")

// Ghost button
Button::new("btn-ghost").ghost().label("Ghost")

// Link button
Button::new("btn-link").link().label("Link")

// Text button
Button::new("btn-text").text().label("Text")
```

### Outline Buttons

Outline style is not a variant itself, but can be combined with other variants.

```rust
Button::new("btn").primary().outline().label("Primary Outline")
Button::new("btn").danger().outline().label("Danger Outline")
```

### Compact Button

The `compact` method reduces the padding of the button for a more condensed appearance.

```rust
// Compact (reduced padding)
Button::new("btn")
    .label("Compact")
    .compact()
```

### Sizeable

The Button supports the [Sizable] trait for different sizes.

```rust
Button::new("btn").xsmall().label("Extra Small")
Button::new("btn").small().label("Small")
Button::new("btn").label("Medium") // default
Button::new("btn").large().label("Large")
```

### With Icons

```rust
use gpui_component::{Icon, IconName};

// Icon before label
Button::new("btn")
    .icon(IconName::Check)
    .label("Confirm")

// Icon only
Button::new("btn")
    .icon(IconName::Search)

// Custom icon size
Button::new("btn")
    .icon(Icon::new(IconName::Heart))
    .label("Like")
```

### Button States

There have `disabled`, `loading`, `selected` state for buttons to indicate different statuses.

```rust
// Disabled
Button::new("btn")
    .label("Disabled")
    .disabled(true)

// Loading
Button::new("btn")
    .label("Loading")
    .loading(true)

// Selected
Button::new("btn")
    .label("Selected")
    .selected(true)
```

## Button Group

```rust
ButtonGroup::new("btn-group")
    .child(Button::new("btn1").label("One"))
    .child(Button::new("btn2").label("Two"))
    .child(Button::new("btn3").label("Three"))
```

### Toggle Button Group

```rust
ButtonGroup::new("toggle-group")
    .multiple(true) // Allow multiple selections
    .child(Button::new("btn1").label("Option 1").selected(true))
    .child(Button::new("btn2").label("Option 2"))
    .child(Button::new("btn3").label("Option 3"))
    .on_click(|selected_indices, _, _| {
        println!("Selected: {:?}", selected_indices);
    })
```

## Custom Variant

```rust
use gpui_component::button::ButtonCustomVariant;

let custom = ButtonCustomVariant::new(cx)
    .color(cx.theme().magenta)
    .foreground(cx.theme().primary_foreground)
    .border(cx.theme().magenta)
    .hover(cx.theme().magenta.opacity(0.1))
    .active(cx.theme().magenta);

Button::new("custom-btn")
    .custom(custom)
    .label("Custom Button")
```

## API Reference

- [Button]
- [ButtonGroup]
- [ButtonCustomVariant]

## Examples

### With Tooltip

```rust
Button::new("btn")
    .label("Hover me")
    .tooltip("This is a helpful tooltip")
```

### Custom Children

```rust
Button::new("btn")
    .child(
        h_flex()
            .items_center()
            .gap_2()
            .child("Custom Content")
            .child(IconName::ChevronDown)
            .child(IconName::Eye)
    )
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[ButtonGroup]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonGroup.html
[ButtonCustomVariant]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.ButtonCustomVariant.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
