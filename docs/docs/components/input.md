---
title: Input
description: Text input component with validation, masking, and various features.
---

# Input

A flexible text input component with support for validation, masking, prefix/suffix elements, and different states.

## Import

```rust
use gpui_component::input::{InputState, Input};
```

## Usage

### Basic Input

```rust
let input = cx.new(|cx| InputState::new(window, cx));

Input::new(&input)
```

### With Placeholder

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter your name...")
);

Input::new(&input)
```

### With Default Value

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .default_value("John Doe")
);

Input::new(&input)
```

### Cleanable Input

```rust
Input::new(&input)
    .cleanable(true) // Show clear button when input has value
```

### With Prefix and Suffix

```rust
use gpui_component::{Icon, IconName};

// With prefix icon
Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())

// With suffix button
Input::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )

// With both
Input::new(&input)
    .prefix(Icon::new(IconName::Search).small())
    .suffix(Button::new("btn").ghost().icon(IconName::Info).xsmall())
```

### Password Input (Masked)

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .masked(true)
        .default_value("password123")
);

Input::new(&input)
    .mask_toggle() // Shows toggle button to reveal password
```

### Input Sizes

```rust
Input::new(&input).large()
Input::new(&input) // medium (default)
Input::new(&input).small()
```

### Disabled Input

```rust
Input::new(&input).disabled(true)
```

### Clean on ESC

```rust
let input = cx.new(|cx|
    InputState::new(window, cx)
        .clean_on_escape() // Clear input when ESC is pressed
);

Input::new(&input)
```

### Input Validation

```rust
// Validate float numbers
let input = cx.new(|cx|
    InputState::new(window, cx)
        .validate(|s, _| s.parse::<f32>().is_ok())
);

// Regex pattern validation
let input = cx.new(|cx|
    InputState::new(window, cx)
        .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
);
```

### Input Masking

```rust
// Phone number mask
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("(999)-999-9999")
);

// Custom pattern: AAA-###-AAA (A=letter, #=digit, 9=digit optional)
let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern("AAA-###-AAA")
);

// Number with thousands separator
use gpui_component::input::MaskPattern;

let input = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(3),
        })
);
```

### Handle Input Events

```rust
let input = cx.new(|cx| InputState::new(window, cx));

cx.subscribe_in(&input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            println!("Input changed: {}", text);
        }
        InputEvent::PressEnter { secondary } => {
            println!("Enter pressed, secondary: {}", secondary);
        }
        InputEvent::Focus => println!("Input focused"),
        InputEvent::Blur => println!("Input blurred"),
    }
});
```

### Custom Appearance

```rust
// Without default styling
Input::new(&input).appearance(false)

// Use in custom container
div()
    .border_b_2()
    .px_6()
    .py_3()
    .border_color(cx.theme().border)
    .bg(cx.theme().secondary)
    .child(Input::new(&input).appearance(false))
```

## Examples

### Search Input

```rust
let search = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Search...")
);

Input::new(&search)
    .prefix(Icon::new(IconName::Search).small())
```

### Currency Input

```rust
let amount = cx.new(|cx|
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2),
        })
);

div()
    .child(Input::new(&amount))
    .child(format!("Value: {}", amount.read(cx).value()))
```

### Form with Multiple Inputs

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

v_flex()
    .gap_3()
    .child(Input::new(&self.name_input))
    .child(Input::new(&self.email_input))
```
