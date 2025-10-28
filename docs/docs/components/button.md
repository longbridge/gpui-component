---
title: Button
description: Displays a button or a component that looks like a button.
---

# Button

A versatile button component with multiple variants, sizes, and states. Supports icons, loading states, and can be grouped together.

## Import

```rust
use gpui_component::button::{Button, ButtonGroup, DropdownButton};
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

### Button Variants

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

```rust
Button::new("btn").primary().outline().label("Primary Outline")
Button::new("btn").danger().outline().label("Danger Outline")
```

### Button Sizes

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

// Compact (reduced padding)
Button::new("btn")
    .label("Compact")
    .compact()
```

### Button Group

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

### Dropdown Button

```rust
use gpui::Corner;

DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu(|menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
            .menu("Option 2", Box::new(MyAction))
            .separator()
            .menu("Option 3", Box::new(MyAction))
    })

// With custom anchor
DropdownButton::new("dropdown")
    .button(Button::new("btn").label("Click Me"))
    .dropdown_menu_with_anchor(Corner::BottomRight, |menu, _, _| {
        menu.menu("Option 1", Box::new(MyAction))
    })
```

### Custom Variant

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

### Button

| Method               | Description                           |
| -------------------- | ------------------------------------- |
| `new(id)`            | Create a new button with the given ID |
| `label(str)`         | Set button label text                 |
| `icon(icon)`         | Add icon to button (before label)     |
| `loading_icon(icon)` | Custom loading icon                   |
| `child(el)`          | Add custom child element              |
| `on_click(fn)`       | Click event handler                   |
| `on_hover(fn)`       | Hover event handler                   |
| `disabled(bool)`     | Set disabled state                    |
| `loading(bool)`      | Set loading state                     |
| `selected(bool)`     | Set selected state                    |
| `compact()`          | Reduce padding                        |

### Button Variants

| Method            | Description                            |
| ----------------- | -------------------------------------- |
| `primary()`       | Primary button style                   |
| `danger()`        | Danger button style                    |
| `warning()`       | Warning button style                   |
| `success()`       | Success button style                   |
| `info()`          | Info button style                      |
| `ghost()`         | Ghost button style                     |
| `link()`          | Link button style                      |
| `text()`          | Text button style                      |
| `outline()`       | Outline style (combines with variants) |
| `custom(variant)` | Custom variant                         |

### ButtonGroup

| Method           | Description                        |
| ---------------- | ---------------------------------- |
| `new(id)`        | Create a new button group          |
| `child(button)`  | Add button to group                |
| `multiple(bool)` | Allow multiple selections          |
| `compact()`      | Compact spacing                    |
| `outline()`      | Apply outline style to all buttons |
| `disabled(bool)` | Disable all buttons                |
| `on_click(fn)`   | Called with selected indices       |

### DropdownButton

| Method                               | Description                   |
| ------------------------------------ | ----------------------------- |
| `new(id)`                            | Create a new dropdown button  |
| `button(btn)`                        | Set the trigger button        |
| `popup_menu(fn)`                     | Set popup menu builder        |
| `popup_menu_with_anchor(corner, fn)` | Set menu with anchor position |

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
