---
title: Notification
description: Display toast notifications that appear at the top right of the window with auto-dismiss functionality.
---

# Notification

A toast notification system for displaying temporary messages to users. Notifications appear at the top right of the window and can auto-dismiss after a timeout. Supports multiple variants (info, success, warning, error), custom content, titles, and action buttons. Perfect for status updates, confirmations, and user feedback.

## Import

```rust
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::WindowExt;
```

## Usage

### Setup application root view for display of notifications

You need to set up your application's root view to render the notification layer. This is typically done in your main application struct's render method.

The [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) function handles rendering any active modals on top of your app content.

```rust
use gpui_component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification_layer = Root::render_notification_layer(window, cx);


        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
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
use gpui_component::text::TextView;

let markdown_content = r#"
## Custom Notification
- **Feature**: New dashboard available
- **Status**: Ready to use
- [Learn more](https://example.com)
"#;

Notification::new()
    .content(|window, cx| {
        TextView::markdown(
            "custom-content",
            markdown_content,
            window,
            cx,
        )
        .into_any_element()
    })
```

### Unique Notifications

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

### Manual Notification Management

```rust
// Show persistent notification
struct PersistentNotification;

window.push_notification(
    Notification::new()
        .id::<PersistentNotification>()
        .message("Background process running...")
        .autohide(false),
    cx,
);

// Later, dismiss the notification
window.remove_notification::<PersistentNotification>(cx);
```

## API Reference

### Notification Methods

| Method                | Description                                              |
| --------------------- | -------------------------------------------------------- |
| `new()`               | Create a new notification with default settings          |
| `info(message)`       | Create an info notification with blue styling            |
| `success(message)`    | Create a success notification with green styling         |
| `warning(message)`    | Create a warning notification with yellow/orange styling |
| `error(message)`      | Create an error notification with red styling            |
| `message(text)`       | Set the notification message content                     |
| `title(text)`         | Set the notification title (appears above message)       |
| `with_type(type)`     | Set the notification type for styling and icon           |
| `icon(icon)`          | Set a custom icon (overrides type default icon)          |
| `autohide(bool)`      | Control auto-dismiss behavior (default: true)            |
| `id<T>()`             | Set unique type-based ID for notification                |
| `id1<T>(key)`         | Set unique type + element ID for notification            |
| `on_click(callback)`  | Set click handler for the notification                   |
| `action(builder)`     | Add an action button to the notification                 |
| `content(builder)`    | Set custom content instead of title/message              |
| `dismiss(window, cx)` | Manually dismiss the notification                        |

### NotificationType Variants

| Type      | Description            | Default Icon  | Theme Color   |
| --------- | ---------------------- | ------------- | ------------- |
| `Info`    | Informational messages | Info          | Blue          |
| `Success` | Success confirmations  | CircleCheck   | Green         |
| `Warning` | Warning messages       | TriangleAlert | Yellow/Orange |
| `Error`   | Error messages         | CircleX       | Red           |

### Window Extensions

| Method                                       | Description                 |
| -------------------------------------------- | --------------------------- |
| `window.push_notification(notification, cx)` | Show a notification         |
| `window.remove_notification::<T>(cx)`        | Remove notification by type |

### Auto-hide Behavior

- **Default timeout**: 5 seconds
- **Auto-hide enabled**: Notification dismisses automatically
- **Auto-hide disabled**: Notification persists until manually closed
- **Hover interaction**: Auto-hide pauses while hovering over notification area

## Examples

### Form Validation Error

```rust
Notification::error("Please correct the following errors before submitting.")
    .title("Validation Failed")
    .autohide(false)
    .action(|_, cx| {
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
use gpui_component::text::TextView;

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
        TextView::markdown("results", results_content, window, cx)
            .into_any_element()
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

## Positioning

Notifications appear in a fixed position at the top right of the window:

- **Position**: `absolute().top_4().right_4()`
- **Stacking**: Newer notifications appear below existing ones
- **Max visible**: Up to 10 notifications shown at once
- **Animation**: Slide down on show, slide right on dismiss
- **Hover expand**: List expands when hovering over notification area

## Animation and Timing

### Show Animation

- **Duration**: 0.25 seconds
- **Easing**: Cubic bezier (0.4, 0, 0.2, 1)
- **Effect**: Slides down and fades in

### Dismiss Animation

- **Duration**: 0.15 seconds
- **Easing**: Cubic bezier (0.4, 0, 0.2, 1)
- **Effect**: Slides right and fades out

### Auto-hide Timing

- **Default delay**: 5 seconds after show
- **Hover pause**: Timer pauses while hovering over notification area
- **Manual dismiss**: Immediate when close button clicked

## Best Practices

### Content Guidelines

- Keep titles concise and descriptive (1-3 words)
- Write clear, actionable messages
- Use appropriate notification types for content
- Provide specific error messages with next steps

### UX Guidelines

- Use auto-hide for confirmations and status updates
- Disable auto-hide for errors requiring user action
- Include action buttons for actionable notifications
- Avoid showing too many notifications simultaneously

### Performance Considerations

- Unique IDs prevent duplicate notifications
- Auto-dismiss reduces notification buildup
- Limit notification frequency to avoid overwhelming users
- Clean up notification subscriptions properly

### Timing Recommendations

- **Success/Info**: Auto-hide after 5 seconds (default)
- **Warnings**: Auto-hide after 7-10 seconds or require action
- **Errors**: Disable auto-hide, require user acknowledgment
- **Progress updates**: Disable auto-hide, update in place
