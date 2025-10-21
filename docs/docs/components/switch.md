---
title: Switch
description: A control that allows the user to toggle between checked and not checked.
---

# Switch

A toggle switch component for binary on/off states. Features smooth animations, different sizes, labels, disabled state, and customizable positioning.

## Import

```rust
use gpui_component::switch::Switch;
```

## Usage

### Basic Switch

```rust
Switch::new("my-switch")
    .checked(false)
    .on_click(|checked, _, _| {
        println!("Switch is now: {}", checked);
    })
```

### Controlled Switch

```rust
struct MyView {
    is_enabled: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Switch::new("switch")
            .checked(self.is_enabled)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.is_enabled = *checked;
                cx.notify();
            }))
    }
}
```

### With Label

```rust
Switch::new("notifications")
    .label("Enable notifications")
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Notifications: {}", if *checked { "enabled" } else { "disabled" });
    })
```

### Label Positioning

```rust
use gpui_component::Side;

// Label on the right (default)
Switch::new("switch1")
    .label("Right label")
    .label_side(Side::Right)

// Label on the left
Switch::new("switch2")
    .label("Left label")
    .label_side(Side::Left)
```

### Different Sizes

```rust
// Small switch
Switch::new("small-switch")
    .small()
    .label("Small switch")

// Medium switch (default)
Switch::new("medium-switch")
    .label("Medium switch")

// Using explicit size
Switch::new("custom-switch")
    .with_size(Size::Small)
    .label("Custom size")
```

### Disabled State

```rust
// Disabled unchecked
Switch::new("disabled-off")
    .label("Disabled (off)")
    .disabled(true)
    .checked(false)

// Disabled checked
Switch::new("disabled-on")
    .label("Disabled (on)")
    .disabled(true)
    .checked(true)
```

### With Tooltip

```rust
Switch::new("switch")
    .label("Airplane mode")
    .tooltip("Enable airplane mode to disable all wireless connections")
    .checked(false)
```

## API Reference

### Switch

| Method             | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| `new(id)`          | Create a new switch with the given ID                       |
| `checked(bool)`    | Set the checked/toggled state                               |
| `label(text)`      | Set label text for the switch                               |
| `label_side(side)` | Position label (Side::Left or Side::Right)                  |
| `disabled(bool)`   | Set disabled state                                          |
| `tooltip(text)`    | Add tooltip text                                            |
| `on_click(fn)`     | Callback when clicked, receives `&bool` (new checked state) |

### Styling

Implements `Sizable` and `Disableable` traits:

- `small()` - Small switch size (28x16px toggle area)
- `medium()` - Medium switch size (36x20px toggle area, default)
- `with_size(size)` - Set explicit size
- `disabled(bool)` - Disabled state

### Styling Properties

The switch can also be styled using GPUI's styling methods:

- `w(width)` - Custom width
- `h(height)` - Custom height
- Standard margin, padding, and positioning methods

## Examples

### Settings Panel

```rust
struct SettingsView {
    marketing_emails: bool,
    security_emails: bool,
    push_notifications: bool,
}

v_flex()
    .gap_4()
    .child(
        // Setting with description
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .child(Label::new("Marketing emails").text_lg())
                            .child(
                                Label::new("Receive emails about new products and features")
                                    .text_color(theme.muted_foreground)
                            )
                    )
                    .child(
                        Switch::new("marketing")
                            .checked(self.marketing_emails)
                            .on_click(cx.listener(|view, checked, _, cx| {
                                view.marketing_emails = *checked;
                                cx.notify();
                            }))
                    )
            )
    )
    .child(
        // Simple setting
        h_flex()
            .items_center()
            .justify_between()
            .child(Label::new("Push notifications"))
            .child(
                Switch::new("push")
                    .checked(self.push_notifications)
                    .on_click(cx.listener(|view, checked, _, cx| {
                        view.push_notifications = *checked;
                        cx.notify();
                    }))
            )
    )
```

### Compact Settings List

```rust
v_flex()
    .gap_3()
    .child(
        Switch::new("wifi")
            .label("Wi-Fi")
            .label_side(Side::Left)
            .checked(true)
            .small()
    )
    .child(
        Switch::new("bluetooth")
            .label("Bluetooth")
            .label_side(Side::Left)
            .checked(false)
            .small()
    )
    .child(
        Switch::new("airplane")
            .label("Airplane Mode")
            .label_side(Side::Left)
            .checked(false)
            .disabled(true)
            .small()
    )
```

### Form Integration

```rust
struct FormData {
    subscribe_newsletter: bool,
    enable_notifications: bool,
    remember_me: bool,
}

v_flex()
    .gap_4()
    .p_4()
    .border_1()
    .border_color(theme.border)
    .rounded(theme.radius)
    .child(
        Switch::new("newsletter")
            .label("Subscribe to newsletter")
            .checked(self.subscribe_newsletter)
            .tooltip("Receive monthly updates about new features")
            .on_click(cx.listener(|view, checked, _, cx| {
                view.subscribe_newsletter = *checked;
                cx.notify();
            }))
    )
    .child(
        Switch::new("notifications")
            .label("Enable notifications")
            .checked(self.enable_notifications)
            .on_click(cx.listener(|view, checked, _, cx| {
                view.enable_notifications = *checked;
                cx.notify();
            }))
    )
    .child(
        Switch::new("remember")
            .label("Remember me")
            .checked(self.remember_me)
            .small()
            .on_click(cx.listener(|view, checked, _, cx| {
                view.remember_me = *checked;
                cx.notify();
            }))
    )
```

### Custom Styling

```rust
Switch::new("custom")
    .label("Custom styled switch")
    .w(px(200.))
    .checked(true)
    .on_click(|checked, _, _| {
        println!("Custom switch: {}", checked);
    })
```

## Accessibility

- Keyboard navigation with Tab
- Toggle with Space or Enter
- Clear focus indicators
- Disabled switches cannot be focused
- Label text announced by screen readers
- Tooltip content announced when present
- State changes announced to assistive technology
- Follows ARIA switch role semantics

## Animation

The switch features smooth animations:

- **Toggle animation**: 150ms duration when switching states
- **Background color transition**: Changes from switch color to primary color
- **Position animation**: Smooth movement of the toggle indicator
- **Disabled state**: Animations are disabled when the switch is disabled
