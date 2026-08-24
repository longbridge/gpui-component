---
title: Message
description: Compose sender identity, metadata, rich content, and actions into an aligned chat message.
---

# Message

`Message` provides the row structure for chat and conversation interfaces. It owns alignment while applications supply message data and compose existing controls into its named slots.

## Import

```rust
use gpui_component::message::{
    Message, MessageAlignment, MessageContent, MessageFooter, MessageGroup, MessageHeader,
};
```

## Anatomy

```rust
Message::new()
    .avatar(Avatar::new().name("Alice").small())
    .header(
        MessageHeader::new()
            .child("Alice")
            .child("10:24 AM"),
    )
    .content(
        MessageContent::new()
            .child("Can you review this?"),
    )
    .footer(
        MessageFooter::new()
            .child("Read"),
    )
```

The avatar accepts any element, so an application can use `Avatar`, an icon, or another identity treatment. Header, content, and footer are typed slots with arbitrary children. `Message` does not own sender records, timestamps, delivery state, or actions.

## Alignment

Alignment is applied to the complete message and all named slots:

```rust
Message::new()
    .alignment(MessageAlignment::End)
    .avatar(Avatar::new().name("You").small())
    .header(MessageHeader::new().child("You"))
    .content(MessageContent::new().child("Sent message"))
    .footer(MessageFooter::new().child("Delivered"))
```

Use `Start` for incoming messages and `End` for outgoing messages. The same alignment type is also accepted by chat surfaces such as `Bubble`.

## Grouping

`MessageGroup` stacks consecutive messages without adding sender or grouping logic:

```rust
MessageGroup::new()
    .child(first_message)
    .child(second_message)
```

## Rich content and actions

Compose existing components in the slots instead of configuring message-specific copies:

```rust
Message::new()
    .content(
        MessageContent::new()
            .child(Bubble::new().child("Hello"))
            .child(Attachment::new().child(file_content)),
    )
    .footer(
        MessageFooter::new()
            .child(Button::new("reply").label("Reply")),
    )
```

## Custom styling

`Message`, `MessageGroup`, `MessageHeader`, `MessageContent`, and `MessageFooter` all implement `Styled`. Style refinements are applied after component defaults:

```rust
Message::new()
    .p_3()
    .rounded(cx.theme().radius_lg)
    .bg(cx.theme().muted.opacity(0.35))
    .header(MessageHeader::new().px_0().child("System"))
    .content(MessageContent::new().child("Archived"))
```

Interactive content should use `Button`, `Link`, or another semantic control inside a slot.

## API reference

- [Message]
- [MessageGroup]
- [MessageHeader]
- [MessageContent]
- [MessageFooter]

[Message]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.Message.html
[MessageGroup]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageGroup.html
[MessageHeader]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageHeader.html
[MessageContent]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageContent.html
[MessageFooter]: https://docs.rs/gpui-component/latest/gpui_component/message/struct.MessageFooter.html
