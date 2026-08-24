---
title: Bubble
description: A styleable chat surface for text, rich content, and reaction controls.
---

# Bubble

`Bubble` is the visible message surface used inside a chat message. It accepts arbitrary children, supports leading or trailing alignment, and provides filled, outline, and ghost treatments.

## Import

```rust
use gpui_component::{
    bubble::{Bubble, BubbleReactionSide, BubbleReactions, BubbleVariant},
    message::MessageAlignment,
};
```

## Basic usage

```rust
Bubble::new()
    .child("Can you review this?")
```

`Bubble` is both the layout container and the styled surface. A separate `BubbleContent` wrapper is unnecessary in GPUI because child composition and `Styled` refinements already cover that role.

## Alignment

Bubble and Message share one alignment type:

```rust
Bubble::new()
    .alignment(MessageAlignment::End)
    .child("Outgoing message")
```

When a bubble is used inside an end-aligned `Message`, pass the same value to the bubble so standalone and nested layouts behave consistently.

## Variants

```rust
Bubble::new()
    .with_variant(BubbleVariant::Filled)
    .child("Filled")

Bubble::new()
    .with_variant(BubbleVariant::Outline)
    .child("Outline")

Bubble::new()
    .with_variant(BubbleVariant::Ghost)
    .child("Ghost")
```

The shared component keeps only the three surface treatments requested by chat interfaces. Semantic colors such as muted, success, warning, or destructive remain caller styles instead of becoming chat-specific variants.

## Rich content

Any GPUI element can be a direct child:

```rust
Bubble::new().child(
    h_flex()
        .gap_3()
        .child(file_icon)
        .child(file_details),
)
```

## Reactions

`BubbleReactions` owns the edge positioning and reaction-region styling. Use existing `Button` components for interactive reactions:

```rust
Bubble::new()
    .child("Looks good to me.")
    .child(
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

Both public elements implement `Styled`, and caller refinements are applied after defaults:

```rust
Bubble::new()
    .rounded(cx.theme().radius)
    .bg(cx.theme().green.opacity(0.15))
    .text_color(cx.theme().green)
    .border_color(cx.theme().green.opacity(0.35))
    .child("Custom semantic color")
```

## API reference

- [Bubble]
- [BubbleReactions]

[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[BubbleReactions]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleReactions.html
