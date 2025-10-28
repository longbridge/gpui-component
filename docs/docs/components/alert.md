---
title: Alert
description: Displays a callout for user attention.
---

# Alert

A versatile alert component for displaying important messages to users. Supports multiple variants (info, success, warning, error), custom icons, optional titles, closable functionality, and banner mode. Perfect for notifications, status messages, and user feedback.

## Import

```rust
use gpui_component::alert::Alert;
```

## Usage

### Basic Alert

```rust
Alert::new("alert-id", "This is a basic alert message.")
```

### Alert with Title

```rust
Alert::new("alert-with-title", "Your changes have been saved successfully.")
    .title("Success!")
```

### Alert Variants

```rust
// Info alert (blue)
Alert::info("info-alert", "This is an informational message.")
    .title("Information")

// Success alert (green)
Alert::success("success-alert", "Your operation completed successfully.")
    .title("Success!")

// Warning alert (yellow/orange)
Alert::warning("warning-alert", "Please review your settings before proceeding.")
    .title("Warning")

// Error alert (red)
Alert::error("error-alert", "An error occurred while processing your request.")
    .title("Error")
```

### Alert Sizes

```rust
Alert::info("alert", "Message content")
    .with_size(Size::XSmall)    // Extra small
    .title("XSmall Alert")

Alert::info("alert", "Message content")
    .with_size(Size::Small)     // Small
    .title("Small Alert")

Alert::info("alert", "Message content")
    .with_size(Size::Medium)    // Medium (default)
    .title("Medium Alert")

Alert::info("alert", "Message content")
    .with_size(Size::Large)     // Large
    .title("Large Alert")
```

### Closable Alerts

```rust
Alert::info("closable-alert", "This alert can be dismissed.")
    .title("Dismissible")
    .on_close(|_event, _window, _cx| {
        println!("Alert was closed");
        // Handle alert dismissal
    })
```

### Banner Mode

Banner alerts take full width and don't display titles:

```rust
Alert::info("banner-alert", "This is a banner alert that spans the full width.")
    .banner()

Alert::success("banner-success", "Operation completed successfully!")
    .banner()

Alert::warning("banner-warning", "System maintenance scheduled for tonight.")
    .banner()

Alert::error("banner-error", "Service temporarily unavailable.")
    .banner()
```

### Custom Icons

```rust
use gpui_component::IconName;

Alert::new("custom-icon", "Meeting scheduled for tomorrow at 3 PM.")
    .title("Calendar Reminder")
    .icon(IconName::Calendar)
```

### With Markdown Content

We can use `TextView` to render formatted (Markdown or HTML) text within the alert,
for displaying lists, bold text, links, etc.

```rust
use gpui_component::text::TextView;

Alert::error(
    "error-with-markdown",
    TextView::markdown(
        "error-message",
        "Please verify your billing information and try again.\n\
        - Check your card details\n\
        - Ensure sufficient funds\n\
        - Verify billing address",
        window,
        cx,
    ),
)
.title("Payment Failed")
```

### Conditional Visibility

```rust
Alert::info("conditional-alert", "This alert may be hidden.")
    .title("Conditional")
    .visible(should_show_alert) // boolean condition
```

## API Reference

### Alert Methods

| Method                 | Description                                         |
| ---------------------- | --------------------------------------------------- |
| `new(id, message)`     | Create a new alert with default (secondary) variant |
| `info(id, message)`    | Create an info alert with blue styling              |
| `success(id, message)` | Create a success alert with green styling           |
| `warning(id, message)` | Create a warning alert with yellow/orange styling   |
| `error(id, message)`   | Create an error alert with red styling              |
| `title(text)`          | Set the alert title (not shown in banner mode)      |
| `icon(icon)`           | Set a custom icon for the alert                     |
| `banner()`             | Enable banner mode (full width, no title)           |
| `on_close(callback)`   | Make alert closable with close button               |
| `visible(bool)`        | Control alert visibility                            |
| `with_size(size)`      | Set alert size (XSmall, Small, Medium, Large)       |

### Alert Variants

| Variant     | Description                   | Default Icon  |
| ----------- | ----------------------------- | ------------- |
| `Secondary` | Default gray styling          | Info          |
| `Info`      | Blue informational styling    | Info          |
| `Success`   | Green success styling         | CircleCheck   |
| `Warning`   | Yellow/orange warning styling | TriangleAlert |
| `Error`     | Red error styling             | CircleX       |

### Size Options

| Size     | Padding   | Gap  | Border Radius |
| -------- | --------- | ---- | ------------- |
| `XSmall` | 12px/6px  | 6px  | Default       |
| `Small`  | 12px/8px  | 6px  | Default       |
| `Medium` | 16px/10px | 12px | Default       |
| `Large`  | 20px/14px | 12px | Large         |

## Examples

### Form Validation Errors

```rust
Alert::error(
    "validation-error",
    "Please correct the following errors before submitting:\n\
    - Email address is required\n\
    - Password must be at least 8 characters\n\
    - Terms of service must be accepted"
)
.title("Validation Failed")
.with_size(Size::Medium)
```

### Success Notification

```rust
Alert::success("save-success", "Your profile has been updated successfully.")
    .title("Changes Saved")
    .on_close(|_, _, _| {
        // Auto-dismiss after showing
    })
```

### System Status Banner

```rust
Alert::warning(
    "maintenance-banner",
    "Scheduled maintenance will occur tonight from 2:00 AM to 4:00 AM EST. \
    Some services may be temporarily unavailable."
)
.banner()
.with_size(Size::Large)
```

### Interactive Alert with Custom Action

```rust
Alert::info("update-available", "A new version of the application is available.")
    .title("Update Available")
    .icon(IconName::Download)
    .on_close(cx.listener(|this, _, _, cx| {
        // Handle update or dismiss
        this.handle_update_notification(cx);
    }))
```

### Multi-line Content with Formatting

```rust
Alert::warning(
    "security-alert",
    TextView::markdown(
        "security-content",
        "**Security Notice**: Unusual activity detected on your account.\n\n\
        Recent activity:\n\
        - Login from new device (Chrome on Windows)\n\
        - Location: San Francisco, CA\n\
        - Time: Today at 2:30 PM\n\n\
        If this wasn't you, please [change your password](/) immediately.",
        window,
        cx,
    )
)
.title("Security Alert")
.icon(IconName::Shield)
```
