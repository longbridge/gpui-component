---
title: Clipboard
description: A button component that helps you copy text or other content to your clipboard.
---

# Clipboard

The Clipboard component provides an easy way to copy text or other data to the user's clipboard. It renders as a button with a copy icon that changes to a checkmark when content is successfully copied. The component supports both static values and dynamic content through callback functions.

## Import

```rust
use gpui_component::clipboard::Clipboard;
```

## Usage

### Basic Clipboard

```rust
Clipboard::new("my-clipboard")
    .value("Text to copy")
    .on_copied(|value, window, cx| {
        window.push_notification(format!("Copied: {}", value), cx)
    })
```

### With Dynamic Content

```rust
Clipboard::new("clipboard")
    .content(|_, _| Label::new("Copy this text"))
    .value("Hello, World!")
```

### Using Value Function

For dynamic values that should be computed when the copy action occurs:

```rust
let state = some_state.clone();
Clipboard::new("dynamic-clipboard")
    .value_fn(move |_, cx| {
        state.read(cx).get_current_value()
    })
    .on_copied(|value, window, cx| {
        window.push_notification(format!("Copied: {}", value), cx)
    })
```

### With Custom Content

```rust
use gpui_component::label::Label;

Clipboard::new("custom-clipboard")
    .content(|_, _|
        h_flex()
            .gap_2()
            .child(Label::new("Share URL"))
            .child(Icon::new(IconName::Share))
    )
    .value("https://example.com")
```

### In Input Fields

The Clipboard component is commonly used as a suffix in input fields:

```rust
use gpui_component::input::{InputState, TextInput};

let url_state = cx.new(|cx| InputState::new(window, cx).default_value("https://github.com"));

TextInput::new(&url_state)
    .suffix(
        Clipboard::new("url-clipboard")
            .value_fn({
                let state = url_state.clone();
                move |_, cx| state.read(cx).value()
            })
            .on_copied(|value, window, cx| {
                window.push_notification(format!("URL copied: {}", value), cx)
            })
    )
```

## API Reference

### Clipboard

| Method          | Description                                             |
| --------------- | ------------------------------------------------------- |
| `new(id)`       | Create a new clipboard component with the given ID      |
| `value(str)`    | Set static text to copy to clipboard                    |
| `value_fn(fn)`  | Set dynamic function that returns the value to copy     |
| `content(fn)`   | Set custom content to display alongside the copy button |
| `on_copied(fn)` | Callback executed when content is successfully copied   |

### Method Details

#### `value(value: impl Into<SharedString>)`

Sets a static value that will be copied to the clipboard when the button is clicked.

```rust
Clipboard::new("static")
    .value("Static text to copy")
```

#### `value_fn(fn: impl Fn(&mut Window, &mut App) -> SharedString + 'static)`

Sets a function that will be called to get the value when the copy action occurs. This is useful for dynamic content that may change over time.

```rust
Clipboard::new("dynamic")
    .value_fn(|_, cx| {
        format!("Current time: {}", SystemTime::now())
    })
```

#### `content(fn: impl Fn(&mut Window, &mut App) -> E + 'static)`

Sets custom content to display before the copy button. The content can be any element that implements `IntoElement`.

```rust
Clipboard::new("with-content")
    .content(|_, _| Label::new("Copy me"))
    .value("Hello")
```

#### `on_copied(fn: impl Fn(SharedString, &mut Window, &mut App) + 'static)`

Sets a callback that is executed when content is successfully copied. Receives the copied value as the first parameter.

```rust
Clipboard::new("with-callback")
    .value("Hello")
    .on_copied(|value, window, cx| {
        println!("Copied: {}", value);
        window.push_notification("Copied to clipboard!", cx);
    })
```

## Behavior

### Visual States

The clipboard button has two visual states:

1. **Default State**: Shows a copy icon (IconName::Copy)
2. **Copied State**: Shows a checkmark icon (IconName::Check) for 2 seconds after successful copy

### Copy Process

1. User clicks the clipboard button
2. The component determines the value to copy:
   - If `value_fn` is set, calls the function to get the current value
   - Otherwise, uses the static `value`
3. Writes the value to the system clipboard using `ClipboardItem::new_string()`
4. Changes the button icon to a checkmark
5. Calls the `on_copied` callback if provided
6. After 2 seconds, resets the icon back to the copy icon

### Event Handling

- Click events are handled internally and call `cx.stop_propagation()` to prevent bubbling
- The component is disabled (unclickable) while in the "copied" state
- Uses GPUI's clipboard API (`cx.write_to_clipboard()`) for system integration

## Examples

### Simple Text Copy

```rust
Clipboard::new("simple")
    .value("Hello, World!")
```

### With User Feedback

```rust
Clipboard::new("feedback")
    .content(|_, _| Label::new("API Key"))
    .value("sk-1234567890abcdef")
    .on_copied(|_, window, cx| {
        window.push_notification("API key copied to clipboard", cx)
    })
```

### Form Field Integration

```rust
use gpui_component::{
    input::{InputState, TextInput},
    h_flex, label::Label
};

let api_key = "sk-1234567890abcdef";

h_flex()
    .gap_2()
    .items_center()
    .child(Label::new("API Key:"))
    .child(
        TextInput::new(&input_state)
            .value(api_key)
            .readonly(true)
            .suffix(
                Clipboard::new("api-key-copy")
                    .value(api_key)
                    .on_copied(|_, window, cx| {
                        window.push_notification("API key copied!", cx)
                    })
            )
    )
```

### Dynamic Content Copy

```rust
struct AppState {
    current_url: String,
}

let app_state = cx.new(|_| AppState {
    current_url: "https://example.com".to_string()
});

Clipboard::new("current-url")
    .content(|_, _| Label::new("Share current page"))
    .value_fn({
        let state = app_state.clone();
        move |_, cx| {
            SharedString::from(state.read(cx).current_url.clone())
        }
    })
    .on_copied(|url, window, cx| {
        window.push_notification(format!("Shared: {}", url), cx)
    })
```

## Data Types

The Clipboard component currently supports copying text strings to the clipboard. It uses GPUI's `ClipboardItem::new_string()` method, which handles:

- Plain text strings
- UTF-8 encoded content
- Cross-platform clipboard integration
