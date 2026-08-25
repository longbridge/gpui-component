---
title: Message
description: 将发送者身份、元信息、富内容和操作组合成对齐的聊天消息。
---

# Message

`Message` 为聊天与会话界面提供消息行结构。组件负责整体对齐，应用负责消息数据，并通过具名 slot 组合已有控件。

## 导入

```rust
use gpui_component::message::{
    Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter,
    MessageGroup, MessageHeader,
};
```

## 结构

```rust
Message::new()
    .avatar_slot(
        MessageAvatar::new()
            .child(Avatar::new().name("Alice").size_8()),
    )
    .header(
        MessageHeader::new()
            .child("Alice")
            .child("10:24 AM"),
    )
    .content(
        MessageContent::new()
            .child("可以帮我检查一下吗？"),
    )
    .footer(
        MessageFooter::new()
            .child("已读"),
    )
```

`MessageAvatar` 会保留共享的 `size-8` avatar 尺寸等级；存在 footer 时，它仍与可见消息表面对齐。`.avatar(...)` builder 会把任意 element 自动包装进该 slot，作为便利写法。Header 与 Footer 默认使用 `px-3` 水平内容间距；通过 `MessageContent::bubble(...)` 加入 Ghost Bubble 后，两者会自动去掉缩进。在任一 slot 上显式调用 `.content_inset(...)` 时，显式配置优先；已有的 `.px_0()` refinement 仍然支持。Avatar 尺寸、文字和间距均跟随应用的 `rem` 缩放体系。Header、Content 与 Footer 是可容纳任意 child 的具名 slot。`Message` 不持有发送者记录、时间戳、送达状态或操作逻辑。

## 对齐

对齐会统一作用于整条消息及其具名 slot：

```rust
Message::new()
    .alignment(MessageAlignment::End)
    .avatar(Avatar::new().name("You").size_8())
    .header(MessageHeader::new().child("你"))
    .content(MessageContent::new().child("已发送的消息"))
    .footer(MessageFooter::new().child("已送达"))
```

接收消息使用 `Start`，发送消息使用 `End`。`Bubble` 等聊天表面也复用同一个对齐类型。

## 消息分组

`MessageGroup` 用于堆叠同一发送者的连续消息，不额外引入发送者或分组逻辑：

```rust
MessageGroup::new()
    .child(first_message)
    .child(second_message)
```

## 富内容与操作

在 slot 中组合现有组件，无需创建消息专用的重复控件：

```rust
Message::new()
    .content(
        MessageContent::new()
            .bubble(Bubble::new().child("你好"))
            .child(
                Attachment::new()
                    .content(AttachmentContent::new().child(file_content)),
            ),
    )
    .footer(
        MessageFooter::new()
            .child(Button::new("reply").label("回复")),
    )
```

## Ghost surface 与 stack 样式

可以独立调整具名 slot 所在的 inner stack。通过具名方法加入 Ghost Bubble
后，Header 与 Footer 会自动去掉默认缩进：

```rust
use gpui::{StyleRefinement, Styled as _};
use gpui_component::bubble::{Bubble, BubbleVariant};

Message::new()
    .with_stack_style(StyleRefinement::default().gap_3())
    .header(
        MessageHeader::new()
            .child("系统")
            .child("刚刚"),
    )
    .content(
        MessageContent::new()
            .bubble(
                Bubble::new()
                    .with_variant(BubbleVariant::Ghost)
                    .child("会话已归档。"),
            ),
    )
    .footer(
        MessageFooter::new()
            .child("无需进一步操作"),
    )
```

调用 `.content_inset(true)` 可以让指定 slot 在 Ghost Bubble 周围继续保留缩进，
调用 `.content_inset(false)` 可以为其他内容显式去掉缩进。原有的 `.child(...)`
继续接受任意 element；由于它会擦除具体类型，需要自动继承 Bubble 变体时应使用
`.bubble(...)`。

## 自定义样式

`Message`、`MessageGroup`、`MessageAvatar`、`MessageHeader`、`MessageContent` 与 `MessageFooter` 都实现了 `Styled`。调用方样式会在组件默认样式之后应用。具名 slot 之间的 stack 使用 `with_stack_style(...)` 调整；slot 自身的表面和 typography 使用各自的 `Styled` refinement：

```rust
Message::new()
    .p_3()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().muted.opacity(0.35))
    .header(MessageHeader::new().px_0().child("系统"))
    .content(MessageContent::new().child("会话已归档"))
```

需要交互时，请在 slot 中使用 `Button`、`Link` 或其他具备明确语义的控件。

## API 参考

- [Message]
- [MessageGroup]
- [MessageAvatar]
- [MessageHeader]
- [MessageContent]
- [MessageFooter]

[Message]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.Message.html
[MessageGroup]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageGroup.html
[MessageAvatar]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageAvatar.html
[MessageHeader]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageHeader.html
[MessageContent]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageContent.html
[MessageFooter]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageFooter.html
