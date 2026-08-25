---
title: Message
description: Compose sender identity, metadata, rich content, and actions into an aligned chat message.
---

# Message

`Message` provides the row structure for chat and conversation interfaces. It owns alignment while applications supply message data and compose existing controls into its named slots.

## Import

```rust
use gpui_component::message::{
    Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter,
    MessageGroup, MessageHeader,
};
```

## Anatomy

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
            .child("Can you review this?"),
    )
    .footer(
        MessageFooter::new()
            .child("Read"),
    )
```

`MessageAvatar` reserves the shared `size-8` avatar baseline and keeps the avatar aligned with the visible surface when a footer is present. The `.avatar(...)` builder wraps any element in this slot as a convenience. Header, content, and footer are typed slots with arbitrary children. Header and footer use the `px-3` content inset by default; a ghost surface added through `MessageContent::bubble(...)` removes both insets automatically. An explicit `.content_inset(...)` on either slot takes precedence, and existing `.px_0()` refinements remain supported. Avatar geometry, typography, and spacing follow the application's `rem` scale. `Message` does not own sender records, timestamps, delivery state, or actions.

## Alignment

Alignment is applied to the complete message and all named slots:

```rust
Message::new()
    .alignment(MessageAlignment::End)
    .avatar(Avatar::new().name("You").size_8())
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
            .bubble(Bubble::new().child("Hello"))
            .child(
                Attachment::new()
                    .content(AttachmentContent::new().child(file_content)),
            ),
    )
    .footer(
        MessageFooter::new()
            .child(Button::new("reply").label("Reply")),
    )
```

## Ghost surfaces and stack styling

The inner stack can be refined independently from the message row. A typed
ghost bubble automatically removes the default header and footer insets:

```rust
use gpui::{StyleRefinement, Styled as _};
use gpui_component::bubble::{Bubble, BubbleVariant};

Message::new()
    .with_stack_style(StyleRefinement::default().gap_3())
    .header(
        MessageHeader::new()
            .child("System")
            .child("Just now"),
    )
    .content(
        MessageContent::new()
            .bubble(
                Bubble::new()
                    .with_variant(BubbleVariant::Ghost)
                    .child("The conversation has been archived."),
            ),
    )
    .footer(
        MessageFooter::new()
            .child("No further action required"),
    )
```

Call `.content_inset(true)` to keep an individual slot inset even around a
ghost bubble, or `.content_inset(false)` to remove it for any other content.
The existing `.child(...)` builder still accepts arbitrary elements; because
it erases their concrete type, use `.bubble(...)` when variant-aware layout
inheritance is desired.

## Custom styling

`Message`, `MessageGroup`, `MessageAvatar`, `MessageHeader`, `MessageContent`, and `MessageFooter` all implement `Styled`. Style refinements are applied after component defaults. Use `with_stack_style(...)` for the named-slot stack; use the slot's own `Styled` refinement for its surface and typography:

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
