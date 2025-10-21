---
title: Toggle
description: A button-style toggle component for binary on/off or selected states.
---

# Toggle

A button-style toggle component that represents on/off or selected states. Unlike a traditional switch, toggles appear as buttons that can be pressed in or out. They're perfect for toolbar buttons, filter options, or any binary choice that benefits from a button-like appearance.

## Import

```rust
use gpui_component::button::{Toggle, ToggleGroup};
```

## Usage

### Basic Toggle

```rust
Toggle::label("Toggle me")
    .id("basic-toggle")
    .checked(false)
    .on_change(|checked, _, _| {
        println!("Toggle is now: {}", checked);
    })
```

### Icon Toggle

```rust
use gpui_component::IconName;

Toggle::icon(IconName::Eye)
    .id("visibility-toggle")
    .checked(true)
    .on_change(|checked, _, _| {
        println!("Visibility: {}", if *checked { "shown" } else { "hidden" });
    })
```

### Controlled Toggle

```rust
struct MyView {
    is_active: bool,
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Toggle::label("Active")
            .id("active-toggle")
            .checked(self.is_active)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.is_active = *checked;
                cx.notify();
            }))
    }
}
```

### Toggle Variants

```rust
// Ghost toggle (default)
Toggle::label("Ghost")
    .id("ghost-toggle")
    .ghost()

// Outline toggle
Toggle::label("Outline")
    .id("outline-toggle")
    .outline()
```

### Different Sizes

```rust
// Extra small
Toggle::icon(IconName::Star)
    .id("xs-toggle")
    .xsmall()

// Small
Toggle::label("Small")
    .id("small-toggle")
    .small()

// Medium (default)
Toggle::label("Medium")
    .id("medium-toggle")

// Large
Toggle::label("Large")
    .id("large-toggle")
    .large()
```

### Disabled State

```rust
// Disabled unchecked
Toggle::label("Disabled")
    .id("disabled-toggle")
    .disabled(true)
    .checked(false)

// Disabled checked
Toggle::label("Selected (Disabled)")
    .id("disabled-checked-toggle")
    .disabled(true)
    .checked(true)
```

## Toggle vs Switch

| Feature                | Toggle                                      | Switch                                    |
| ---------------------- | ------------------------------------------- | ----------------------------------------- |
| **Appearance**         | Button-like, can be pressed in/out          | Traditional switch with sliding indicator |
| **Use Cases**          | Toolbar buttons, filters, binary options    | Settings, preferences, on/off states      |
| **Visual Style**       | Rectangular button shape                    | Rounded switch track with thumb           |
| **State Indication**   | Background color change, pressed appearance | Position of sliding thumb                 |
| **Multiple Selection** | Supports groups with multiple selection     | Individual switches only                  |

**Use Toggle when you want:**

- Button-like appearance for binary states
- Grouping multiple related options
- Toolbar or filter interfaces
- Options that feel like "selections" rather than "settings"

**Use Switch when you want:**

- Traditional on/off control appearance
- Settings or preferences interface
- Clear visual indication of state with sliding animation
- Individual boolean controls

## Integration with ToggleGroup

Toggle buttons can be grouped together using `ToggleGroup` for related options:

### Basic Toggle Group

```rust
ToggleGroup::new("filter-group")
    .child(Toggle::icon(IconName::Bell))
    .child(Toggle::icon(IconName::Bot))
    .child(Toggle::icon(IconName::Inbox))
    .child(Toggle::label("Other"))
    .on_change(|checkeds, _, _| {
        println!("Selected toggles: {:?}", checkeds);
    })
```

### Toggle Group with Controlled State

```rust
struct FilterView {
    notifications: bool,
    bots: bool,
    inbox: bool,
    other: bool,
}

impl Render for FilterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ToggleGroup::new("filters")
            .child(Toggle::icon(IconName::Bell).checked(self.notifications))
            .child(Toggle::icon(IconName::Bot).checked(self.bots))
            .child(Toggle::icon(IconName::Inbox).checked(self.inbox))
            .child(Toggle::label("Other").checked(self.other))
            .on_change(cx.listener(|view, checkeds, _, cx| {
                view.notifications = checkeds[0];
                view.bots = checkeds[1];
                view.inbox = checkeds[2];
                view.other = checkeds[3];
                cx.notify();
            }))
    }
}
```

### Toggle Group Variants and Sizes

```rust
// Outline variant, small size
ToggleGroup::new("compact-filters")
    .outline()
    .small()
    .child(Toggle::icon(IconName::Filter))
    .child(Toggle::icon(IconName::Sort))
    .child(Toggle::icon(IconName::Search))

// Ghost variant (default), extra small
ToggleGroup::new("mini-toolbar")
    .xsmall()
    .child(Toggle::icon(IconName::Bold))
    .child(Toggle::icon(IconName::Italic))
    .child(Toggle::icon(IconName::Underline))
```

## Event Handling

### Individual Toggle Events

```rust
Toggle::label("Subscribe")
    .id("subscribe-toggle")
    .on_change(|checked, window, cx| {
        if *checked {
            // Handle subscription logic
            println!("Subscribed!");
        } else {
            // Handle unsubscription logic
            println!("Unsubscribed!");
        }
    })
```

### Toggle Group Events

The `on_change` callback for `ToggleGroup` receives a `Vec<bool>` representing the state of each toggle:

```rust
ToggleGroup::new("options")
    .child(Toggle::label("Option 1"))
    .child(Toggle::label("Option 2"))
    .child(Toggle::label("Option 3"))
    .on_change(|states, _, _| {
        for (i, checked) in states.iter().enumerate() {
            if *checked {
                println!("Option {} is selected", i + 1);
            }
        }
    })
```

## API Reference

### Toggle

| Method           | Description                                                 |
| ---------------- | ----------------------------------------------------------- |
| `label(str)`     | Create toggle with text label                               |
| `icon(icon)`     | Create toggle with icon                                     |
| `id(id)`         | Set element ID and make interactive                         |
| `checked(bool)`  | Set checked/selected state                                  |
| `disabled(bool)` | Set disabled state                                          |
| `on_change(fn)`  | Callback when clicked, receives `&bool` (new checked state) |

### Toggle Variants

| Method      | Description                                      |
| ----------- | ------------------------------------------------ |
| `ghost()`   | Ghost variant (default) - transparent background |
| `outline()` | Outline variant - visible border                 |

### Toggle Sizing

Implements `Sizable` trait:

- `xsmall()` - Extra small toggle (20x20px)
- `small()` - Small toggle (24x24px)
- `medium()` - Medium toggle (32x32px, default)
- `large()` - Large toggle (36x36px)
- `with_size(size)` - Set explicit size

### ToggleGroup

| Method           | Description                                             |
| ---------------- | ------------------------------------------------------- |
| `new(id)`        | Create a new toggle group with ID                       |
| `child(toggle)`  | Add a toggle to the group                               |
| `children(iter)` | Add multiple toggles to the group                       |
| `on_change(fn)`  | Callback when any toggle changes, receives `&Vec<bool>` |
| `disabled(bool)` | Disable all toggles in the group                        |

### ToggleGroup Styling

Implements `Sizable`, `ToggleVariants`, and `Disableable` traits:

- Size methods apply to all child toggles
- Variant methods apply to all child toggles
- `disabled(bool)` affects the entire group

## Examples

### Toolbar with Toggle Buttons

```rust
struct EditorToolbar {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

h_flex()
    .gap_1()
    .p_2()
    .bg(cx.theme().background)
    .border_1()
    .border_color(cx.theme().border)
    .child(
        ToggleGroup::new("formatting")
            .small()
            .child(Toggle::icon(IconName::Bold).checked(self.bold))
            .child(Toggle::icon(IconName::Italic).checked(self.italic))
            .child(Toggle::icon(IconName::Underline).checked(self.underline))
            .child(Toggle::icon(IconName::Strikethrough).checked(self.strikethrough))
            .on_change(cx.listener(|view, states, _, cx| {
                view.bold = states[0];
                view.italic = states[1];
                view.underline = states[2];
                view.strikethrough = states[3];
                cx.notify();
            }))
    )
```

### Filter Interface

```rust
struct FilterPanel {
    show_completed: bool,
    show_pending: bool,
    show_cancelled: bool,
    show_urgent: bool,
}

v_flex()
    .gap_3()
    .p_4()
    .child(Label::new("Filter by status"))
    .child(
        ToggleGroup::new("status-filters")
            .outline()
            .child(Toggle::label("Completed").checked(self.show_completed))
            .child(Toggle::label("Pending").checked(self.show_pending))
            .child(Toggle::label("Cancelled").checked(self.show_cancelled))
            .on_change(cx.listener(|view, states, _, cx| {
                view.show_completed = states[0];
                view.show_pending = states[1];
                view.show_cancelled = states[2];
                cx.notify();
            }))
    )
    .child(
        Toggle::label("Show urgent only")
            .id("urgent-filter")
            .checked(self.show_urgent)
            .on_change(cx.listener(|view, checked, _, cx| {
                view.show_urgent = *checked;
                cx.notify();
            }))
    )
```

### Settings with Individual Toggles

```rust
struct NotificationSettings {
    email_notifications: bool,
    push_notifications: bool,
    marketing_emails: bool,
}

v_flex()
    .gap_4()
    .child(
        h_flex()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .child(Label::new("Email notifications"))
                    .child(
                        Label::new("Receive notifications via email")
                            .text_color(cx.theme().muted_foreground)
                            .text_sm()
                    )
            )
            .child(
                Toggle::icon(IconName::Mail)
                    .id("email-notifications")
                    .checked(self.email_notifications)
                    .on_change(cx.listener(|view, checked, _, cx| {
                        view.email_notifications = *checked;
                        cx.notify();
                    }))
            )
    )
    .child(
        h_flex()
            .items_center()
            .justify_between()
            .child(Label::new("Push notifications"))
            .child(
                Toggle::icon(IconName::Bell)
                    .id("push-notifications")
                    .checked(self.push_notifications)
                    .on_change(cx.listener(|view, checked, _, cx| {
                        view.push_notifications = *checked;
                        cx.notify();
                    }))
            )
    )
```

### Multi-select Options

```rust
struct SelectionView {
    selected_categories: Vec<bool>,
}

impl SelectionView {
    fn categories() -> Vec<&'static str> {
        vec!["Technology", "Design", "Business", "Science", "Art"]
    }
}

v_flex()
    .gap_3()
    .child(Label::new("Select categories of interest"))
    .child(
        ToggleGroup::new("categories")
            .children(
                Self::categories()
                    .into_iter()
                    .enumerate()
                    .map(|(i, category)| {
                        Toggle::label(category)
                            .checked(self.selected_categories.get(i).copied().unwrap_or(false))
                    })
            )
            .on_change(cx.listener(|view, states, _, cx| {
                view.selected_categories = states.clone();
                cx.notify();
            }))
    )
```

## Accessibility

- **Keyboard Navigation**: Toggle with Tab, activate with Space or Enter
- **Focus Management**: Clear focus indicators for individual toggles and groups
- **State Announcement**: Screen readers announce toggle state changes
- **Disabled State**: Disabled toggles cannot be focused or activated
- **Semantic Markup**: Uses appropriate ARIA attributes for toggle button semantics
- **Group Semantics**: Toggle groups are properly associated for assistive technology
- **Label Association**: Icon toggles should have accessible labels or tooltips

### Accessibility Best Practices

```rust
// Provide accessible labels for icon-only toggles
Toggle::icon(IconName::Star)
    .id("favorite-toggle")
    .tooltip("Add to favorites")

// Use descriptive labels
Toggle::label("Enable dark mode")
    .id("dark-mode-toggle")
    .checked(self.dark_mode)

// Group related toggles logically
ToggleGroup::new("text-formatting")
    .child(Toggle::icon(IconName::Bold).tooltip("Bold"))
    .child(Toggle::icon(IconName::Italic).tooltip("Italic"))
    .child(Toggle::icon(IconName::Underline).tooltip("Underline"))
```

## Best Practices

1. **Use meaningful labels**: Choose clear, descriptive text for toggle labels
2. **Group related options**: Use ToggleGroup for logically related binary choices
3. **Provide visual feedback**: The checked state should be clearly distinguishable
4. **Consider context**: Use toggles for options that feel like "selections" rather than "settings"
5. **Maintain state consistency**: Ensure toggle state reflects the actual application state
6. **Accessible labels**: Provide tooltips or ARIA labels for icon-only toggles
