---
title: Notification
description: 在窗口右上角显示支持自动消失的 toast 通知。
---

# Notification

Notification 是一个 toast 通知系统，用于向用户显示短暂消息。通知会出现在窗口右上角，并可在超时后自动消失。它支持多种类型、标题、自定义内容和操作按钮，适合状态反馈、确认信息和异步操作提示。

## 导入

```rust
use gpui_component::{
    notification::{Notification, NotificationType},
    WindowExt
};
```

## 用法

### 在根视图中渲染通知层

如果你想显示通知，需要在应用根视图中渲染 notification layer。

[Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) 会将当前激活的通知渲染在应用内容之上。

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
            .children(notification_layer)
    }
}
```

### 基础通知

```rust
window.push_notification("This is a notification.", cx);

Notification::new()
    .message("Your changes have been saved.")
```

### 通知类型

```rust
window.push_notification(
    (NotificationType::Info, "File saved successfully."),
    cx,
);

window.push_notification(
    (NotificationType::Success, "Payment processed successfully."),
    cx,
);

window.push_notification(
    (NotificationType::Warning, "Network connection is unstable."),
    cx,
);

window.push_notification(
    (NotificationType::Error, "Failed to save file. Please try again."),
    cx,
);
```

### 带标题

```rust
Notification::new()
    .title("Update Available")
    .message("A new version of the application is ready to install.")
    .with_type(NotificationType::Info)
```

### 自动隐藏

```rust
Notification::new()
    .message("This notification stays until manually closed.")
    .autohide(false)

Notification::new()
    .message("This will disappear automatically.")
    .autohide(true)
```

### 操作按钮

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
                println!("Retrying connection...");
                this.dismiss(window, cx);
            }))
    })
```

### 可点击通知

```rust
Notification::new()
    .message("Click to view details")
    .on_click(cx.listener(|_, _, _, cx| {
        println!("Notification clicked");
        cx.notify();
    }))
```

### 自定义内容

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

### 唯一通知 ID

如果你要手动管理通知，例如长任务状态或持久警告，可以为通知分配唯一 ID。

```rust
struct UpdateNotification;

Notification::new()
    .id::<UpdateNotification>()
    .message("System update available")
    .autohide(false)

struct TaskNotification;

Notification::warning("Task failed to complete")
    .id1::<TaskNotification>("task-123")
    .title("Task Failed")
```

后续可以通过：

```rust
window.remove_notification::<UpdateNotification>(cx);
```

来移除对应通知。

## 示例

### 表单校验失败

```rust
Notification::error("Please correct the following errors before submitting.")
    .title("Validation Failed")
    .autohide(false)
```

### 文件上传进度

```rust
struct UploadNotification;

window.push_notification(
    Notification::info("Uploading file...")
        .id::<UploadNotification>()
        .title("File Upload")
        .autohide(false),
    cx,
);
```
