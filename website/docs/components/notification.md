---
title: Notification
description: Display toast notifications that appear at the top right of the window with auto-dismiss functionality.
---

# Notification

A toast notification system for displaying temporary messages to users. Notifications appear at the top right of the window and can auto-dismiss after a timeout. Supports multiple variants (info, success, warning, error), custom content, titles, and action buttons. Perfect for status updates, confirmations, and user feedback.

## Import

```rust
use gpui_component::{
    notification::{Notification, NotificationType},
    WindowExt
};
```

## Usage

### Setup application root view for display of notifications

You need to set up your application's root view to render the notification layer. This is typically done in your main application struct's render method.

The [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) function handles rendering any active modals on top of your app content.

```rust
use gpui_component::{TitleBar, Root};

struct Example {}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().child("Hello world!")),
            )
            // Render the notification layer on top of the app content
            .children(notification_layer)
    }
}
```

### Basic Notification

```rust
// Simple string notification
window.push_notification("This is a notification.", cx);

// Using Notification builder
Notification::new()
    .message("Your changes have been saved.")
```

### Notification Types

```rust
// Info notification (blue)
window.push_notification(
    (NotificationType::Info, "File saved successfully."),
    cx,
);

// Success notification (green)
window.push_notification(
    (NotificationType::Success, "Payment processed successfully."),
    cx,
);

// Warning notification (yellow/orange)
window.push_notification(
    (NotificationType::Warning, "Network connection is unstable."),
    cx,
);

// Error notification (red)
window.push_notification(
    (NotificationType::Error, "Failed to save file. Please try again."),
    cx,
);
```

### Notification with Title

```rust
Notification::new()
    .title("Update Available")
    .message("A new version of the application is ready to install.")
    .with_type(NotificationType::Info)
```

### Auto-hide Control

```rust
// Disable auto-hide (manual dismiss only)
Notification::new()
    .message("This notification stays until manually closed.")
    .autohide(false)

// Default auto-hide after 5 seconds
Notification::new()
    .message("This will disappear automatically.")
    .autohide(true) // default
```

### With Action Button

```rust
Notification::new()
    .title("Connection Lost")
    .message("Unable to connect to server.")
    .with_type(NotificationType::Error)
    .autohide(false)
    .action(|_, cx| {
        Button::new("retry")
            .primary()
            .label("Retry")
            .on_click(cx.listener(|this, _, window, cx| {
                // Perform retry action
                println!("Retrying connection...");
                this.dismiss(window, cx);
            }))
    })
```

### Clickable Notifications

```rust
Notification::new()
    .message("Click to view details")
    .on_click(cx.listener(|_, _, _, cx| {
        println!("Notification clicked");
        // Handle notification click
        cx.notify();
    }))
```

### Custom Content

```rust
use gpui_component::text::markdown;

let markdown_content = r#"
## Custom Notification
- **Feature**: New dashboard available
- **Status**: Ready to use
- [Learn more](https://example.com)
"#;

Notification::new()
    .content(|_, window, cx| {
        markdown(markdown_content).into_any_element()
    })
```

### Unique Notifications

When you need to manage notifications manually, such as for long-running processes or persistent alerts, you can use unique IDs to push and remove notifications as needed.

In this case, you can create a special `struct` in local scope, and use `id` methods with this struct to identify the notification.

Then you can push the notification when needed, and later remove it using the same ID.

Like this:

```rust
// Using type-based ID for uniqueness
struct UpdateNotification;

Notification::new()
    .id::<UpdateNotification>()
    .message("System update available")
    .autohide(false)

// Using type + element ID for multiple unique notifications
struct TaskNotification;

Notification::warning("Task failed to complete")
    .id1::<TaskNotification>("task-123")
    .title("Task Failed")
```

Then remove the notification with `window.remove_notification::<UpdateNotification>`, like this:

```rust
// Later, dismiss the notification
window.remove_notification::<UpdateNotification>(cx);
```

## Examples

### Form Validation Error

```rust
Notification::error("Please correct the following errors before submitting.")
    .title("Validation Failed")
    .autohide(false)
    .action(|_, _, cx| {
        Button::new("review")
            .outline()
            .label("Review Form")
            .on_click(cx.listener(|this, _, window, cx| {
                // Navigate to form
                this.dismiss(window, cx);
            }))
    })
```

### File Upload Progress

```rust
struct UploadNotification;

// Start upload notification
window.push_notification(
    Notification::info("Uploading file...")
        .id::<UploadNotification>()
        .title("File Upload")
        .autohide(false),
    cx,
);

// Update to success when complete
window.push_notification(
    Notification::success("File uploaded successfully!")
        .id::<UploadNotification>()
        .title("Upload Complete"),
    cx,
);
```

### System Status Updates

```rust
// Warning about maintenance
Notification::warning("System maintenance will begin in 30 minutes.")
    .title("Scheduled Maintenance")
    .autohide(false)
    .action(|_, cx| {
        Button::new("details")
            .link()
            .label("View Details")
            .on_click(cx.listener(|this, _, window, cx| {
                // Show maintenance details
                this.dismiss(window, cx);
            }))
    })
```

### Batch Operation Results

```rust
use gpui_component::text::markdown;

let results_content = r#"
## Batch Operation Complete

**Processed**: 150 items
**Success**: 147 items
**Failed**: 3 items

[View failed items](/)
"#;

Notification::success("Batch operation completed with some failures.")
    .title("Operation Results")
    .content(|window, cx| {
        markdown(results_content).into_any_element()
    })
    .autohide(false)
```

### Interactive Confirmation

```rust
struct SaveConfirmation;

Notification::new()
    .id::<SaveConfirmation>()
    .title("Unsaved Changes")
    .message("You have unsaved changes. Save before leaving?")
    .autohide(false)
    .action(|_, cx| {
        Button::new("save")
            .primary()
            .label("Save")
            .on_click(cx.listener(|this, _, window, cx| {
                // Perform save
                println!("Saving changes...");
                this.dismiss(window, cx);
            }))
    })
    .on_click(cx.listener(|_, _, _, cx| {
        println!("Save reminder clicked");
        cx.notify();
    }))
```
