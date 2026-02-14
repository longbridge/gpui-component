---
title: NumberInput
description: Number input component with increment/decrement controls and numeric formatting.
---

# NumberInput

A specialized input component for numeric values with built-in increment/decrement buttons and support for min/max values, step values, and number formatting with thousands separators.

## Import

```rust
use gpui_component::input::{InputState, NumberInput, NumberInputEvent, StepAction};
```

## Usage

### Basic Number Input

```rust
let number_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Enter number")
        .default_value("1")
);

NumberInput::new(&number_input)
```

### With Min/Max Validation

```rust
// Integer input with validation
let integer_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Integer value")
        .pattern(Regex::new(r"^\d+$").unwrap()) // Only positive integers
);

NumberInput::new(&integer_input)
```

### With Number Formatting

```rust
use gpui_component::input::MaskPattern;

// Currency input with thousands separator
let currency_input = cx.new(|cx|
    InputState::new(window, cx)
        .placeholder("Amount")
        .mask_pattern(MaskPattern::Number {
            separator: Some(','),
            fraction: Some(2), // 2 decimal places
        })
);

NumberInput::new(&currency_input)
```

### Different Sizes

```rust
// Large size
NumberInput::new(&input).large()

// Medium size (default)
NumberInput::new(&input)

// Small size
NumberInput::new(&input).small()
```

### With Prefix and Suffix

```rust
use gpui_component::{button::{Button, ButtonVariants}, IconName};

// With currency prefix
NumberInput::new(&input)
    .prefix(div().child("$"))

// With info button suffix
NumberInput::new(&input)
    .suffix(
        Button::new("info")
            .ghost()
            .icon(IconName::Info)
            .xsmall()
    )
```

### Disabled State

```rust
NumberInput::new(&input).disabled(true)
```

### Without Default Styling

```rust
// For custom container styling
div()
    .w_full()
    .bg(cx.theme().secondary)
    .rounded(cx.theme().radius)
    .child(NumberInput::new(&input).appearance(false))
```

### Handle Number Input Events

```rust
let number_input = cx.new(|cx| InputState::new(window, cx));
let mut value: i64 = 0;

// Subscribe to input changes
cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        InputEvent::Change => {
            let text = state.read(cx).value();
            if let Ok(new_value) = text.parse::<i64>() {
                view.value = new_value;
            }
        }
        _ => {}
    }
});

// Subscribe to increment/decrement actions
cx.subscribe_in(&number_input, window, |view, state, event, window, cx| {
    match event {
        NumberInputEvent::Step(step_action) => {
            match step_action {
                StepAction::Increment => {
                    view.value += 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
                StepAction::Decrement => {
                    view.value -= 1;
                    state.update(cx, |input, cx| {
                        input.set_value(view.value.to_string(), window, cx);
                    });
                }
            }
        }
    }
});
```

### Programmatic Control

```rust
// Increment programmatically
NumberInput::increment(&number_input, window, cx);

// Decrement programmatically
NumberInput::decrement(&number_input, window, cx);
```

## API Reference

### NumberInput

| Method                         | Description                                |
| ------------------------------ | ------------------------------------------ |
| `new(state)`                   | Create number input with InputState entity |
| `placeholder(str)`             | Set placeholder text                       |
| `size(size)`                   | Set input size (small, medium, large)      |
| `prefix(el)`                   | Add prefix element                         |
| `suffix(el)`                   | Add suffix element                         |
| `appearance(bool)`             | Enable/disable default styling             |
| `disabled(bool)`               | Set disabled state                         |
| `increment(state, window, cx)` | Increment value programmatically           |
| `decrement(state, window, cx)` | Decrement value programmatically           |

### NumberInputEvent

| Event              | Description                        |
| ------------------ | ---------------------------------- |
| `Step(StepAction)` | Increment/decrement button pressed |

### StepAction

| Action      | Description               |
| ----------- | ------------------------- |
| `Increment` | Value should be increased |
| `Decrement` | Value should be decreased |

### InputState (Number-specific methods)

| Method                              | Description                                             |
| ----------------------------------- | ------------------------------------------------------- |
| `pattern(regex)`                    | Set regex pattern for validation (e.g., digits only)    |
| `mask_pattern(MaskPattern::Number)` | Set number formatting with separator and decimal places |
| `value()`                           | Get current display value (formatted)                   |
| `unmask_value()`                    | Get actual numeric value (unformatted)                  |

### MaskPattern::Number

| Field       | Type            | Description                            |
| ----------- | --------------- | -------------------------------------- |
| `separator` | `Option<char>`  | Thousands separator (e.g., ',' or ' ') |
| `fraction`  | `Option<usize>` | Number of decimal places               |

## Keyboard Navigation

| Key         | Action                     |
| ----------- | -------------------------- |
| `↑`         | Increment value            |
| `↓`         | Decrement value            |
| `Tab`       | Navigate to next field     |
| `Shift+Tab` | Navigate to previous field |
| `Enter`     | Submit/confirm value       |
| `Escape`    | Clear input (if enabled)   |

## Examples

### Integer Counter

```rust
struct CounterView {
    counter_input: Entity<InputState>,
    counter_value: i32,
}

impl CounterView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let counter_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("Count")
                .default_value("0")
                .pattern(Regex::new(r"^-?\d+$").unwrap()) // Allow negative integers
        );

        let _subscription = cx.subscribe_in(&counter_input, window, Self::on_number_event);

        Self {
            counter_input,
            counter_value: 0,
        }
    }

    fn on_number_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                self.counter_value += 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                self.counter_value -= 1;
                state.update(cx, |input, cx| {
                    input.set_value(self.counter_value.to_string(), window, cx);
                });
            }
        }
    }
}

// Usage
NumberInput::new(&self.counter_input)
```

### Currency Input

```rust
struct PriceInput {
    price_input: Entity<InputState>,
    price_value: f64,
}

impl PriceInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let price_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("0.00")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
        );

        Self {
            price_input,
            price_value: 0.0,
        }
    }
}

// Usage with currency prefix
h_flex()
    .gap_2()
    .child(div().child("$"))
    .child(NumberInput::new(&self.price_input))
```

### Quantity Selector with Limits

```rust
struct QuantitySelector {
    quantity_input: Entity<InputState>,
    quantity: u32,
    min_quantity: u32,
    max_quantity: u32,
}

impl QuantitySelector {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let min_quantity = 1;
        let max_quantity = 99;

        let quantity_input = cx.new(|cx|
            InputState::new(window, cx)
                .default_value(min_quantity.to_string())
                .pattern(Regex::new(&format!(r"^[{}-{}]\d*$", min_quantity, max_quantity)).unwrap())
        );

        let _subscription = cx.subscribe_in(&quantity_input, window, Self::on_quantity_event);

        Self {
            quantity_input,
            quantity: min_quantity,
            min_quantity,
            max_quantity,
        }
    }

    fn on_quantity_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                if self.quantity < self.max_quantity {
                    self.quantity += 1;
                    state.update(cx, |input, cx| {
                        input.set_value(self.quantity.to_string(), window, cx);
                    });
                }
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                if self.quantity > self.min_quantity {
                    self.quantity -= 1;
                    state.update(cx, |input, cx| {
                        input.set_value(self.quantity.to_string(), window, cx);
                    });
                }
            }
        }
    }
}

// Usage
NumberInput::new(&self.quantity_input).small()
```

### Floating Point Input

```rust
struct FloatInput {
    float_input: Entity<InputState>,
    float_value: f64,
    step: f64,
}

impl FloatInput {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let float_input = cx.new(|cx|
            InputState::new(window, cx)
                .placeholder("0.0")
                .pattern(Regex::new(r"^-?\d*\.?\d*$").unwrap()) // Allow decimal numbers
        );

        Self {
            float_input,
            float_value: 0.0,
            step: 0.1,
        }
    }

    fn on_float_event(
        &mut self,
        state: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(StepAction::Increment) => {
                self.float_value += self.step;
                state.update(cx, |input, cx| {
                    input.set_value(format!("{:.1}", self.float_value), window, cx);
                });
            }
            NumberInputEvent::Step(StepAction::Decrement) => {
                self.float_value -= self.step;
                state.update(cx, |input, cx| {
                    input.set_value(format!("{:.1}", self.float_value), window, cx);
                });
            }
        }
    }
}
```

## Best Practices

1. **Validation**: Always validate numeric input on both client and server side
2. **Range Limits**: Implement min/max validation for user safety
3. **Step Size**: Choose appropriate step values for your use case
4. **Error Handling**: Provide clear feedback for invalid input
5. **Formatting**: Use consistent number formatting across your application
6. **Performance**: Debounce rapid increment/decrement actions if needed
7. **Accessibility**: Always provide proper labels and descriptions
