---
title: Bubble
description: A styleable chat surface for text, rich content, and reaction controls.
---

# Bubble

`Bubble` lays out a conversational surface and its reactions. The root owns alignment and the 80% maximum width, while `BubbleContent` owns the visible surface styling.

## Import

```rust
use gpui_component::{
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide,
        BubbleReactions, BubbleVariant,
    },
    message::MessageAlignment,
};
```

## Basic usage

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(BubbleContent::new().child("Can you review this?"))
```

For short content, direct children are added to the content slot as a convenience:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("Can you review this?")
```

Use an explicit `BubbleContent` when its surface needs custom styling. This keeps layout refinements on `Bubble` separate from colors, padding, radius, and typography on `BubbleContent`.

## Alignment

Bubble and Message share one alignment type:

```rust
Bubble::new()
    .alignment(MessageAlignment::End)
    .child("Outgoing message")
```

Set alignment when a Bubble is used on its own. Inside `MessageContent`, leave it unset to inherit the alignment that `Message` propagates to the slot.

## Variants

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Filled)
    .child("Filled")

Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Secondary)
    .child("Secondary")

Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Muted)
    .child("Muted")

Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Tinted)
    .child("Tinted")

Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Outline)
    .child("Outline")

Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Ghost)
    .child("Ghost")

Bubble::new()
    .alignment(MessageAlignment::Start)
    .with_variant(BubbleVariant::Destructive)
    .child("The request failed.")
```

Variants use semantic theme tokens. `Ghost` removes the frame and content padding and can fill the row; the other variants size to their content up to 80% of the available width.

## Rich content

Any GPUI element can be a direct child:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new().child(
            h_flex()
                .gap_3()
                .child(file_icon)
                .child(file_details),
        ),
    )
```

## Groups

Use `BubbleGroup` for consecutive bubbles from the same sender:

```rust
BubbleGroup::new()
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Muted)
            .child("First"),
    )
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Muted)
            .child("Second"),
    )
```

## Reactions

`BubbleReactions` owns the edge positioning and reaction-region styling. Use existing `Button` components for interactive reactions:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("Looks good to me.")
    .reactions(
        BubbleReactions::new()
            .side(BubbleReactionSide::Bottom)
            .alignment(MessageAlignment::End)
            .child(
                Button::new("like")
                    .ghost()
                    .label("👍 2"),
            ),
    )
```

There is no `BubbleAction` type. `Button` already provides focus, keyboard, disabled, loading, and accessibility behavior.

## Custom styling

Every public part implements `Styled`, and caller refinements are applied after defaults:

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new()
            .rounded(cx.theme().radius)
            .bg(cx.theme().green.opacity(0.15))
            .text_color(cx.theme().green)
            .border_color(cx.theme().green.opacity(0.35))
            .child("Custom semantic color"),
    )
```

## API reference

- [Bubble]
- [BubbleContent]
- [BubbleGroup]
- [BubbleReactions]

[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[BubbleContent]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleContent.html
[BubbleGroup]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleGroup.html
[BubbleReactions]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleReactions.html
