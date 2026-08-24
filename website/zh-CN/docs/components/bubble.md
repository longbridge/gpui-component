---
title: Bubble
description: 可承载文本、富内容和 reaction 控件的聊天消息表面。
---

# Bubble

`Bubble` 是聊天消息中可见的消息表面。它可以容纳任意 child，支持起始端或末端对齐，并提供 filled、outline 与 ghost 三种样式。

## 导入

```rust
use gpui_component::{
    bubble::{Bubble, BubbleReactionSide, BubbleReactions, BubbleVariant},
    message::MessageAlignment,
};
```

## 基础用法

```rust
Bubble::new()
    .child("可以帮我检查一下吗？")
```

`Bubble` 同时承担布局容器和可见表面。GPUI 已可通过 child 组合与 `Styled` refinement 覆盖样式，因此无需额外增加 `BubbleContent` 包装层。

## 对齐

Bubble 与 Message 共用同一个对齐类型：

```rust
Bubble::new()
    .alignment(MessageAlignment::End)
    .child("发出的消息")
```

Bubble 独立使用时可以显式设置对齐。放在 `MessageContent` 中时可保持未设置，由 `Message` 将对齐传播给该 slot。

## 样式变体

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

共享组件只保留聊天界面要求的三种表面样式。Muted、success、warning、destructive 等语义颜色由调用方通过样式表达，不扩展成聊天专用 variant。

## 富内容

任意 GPUI element 都可以作为直接 child：

```rust
Bubble::new().child(
    h_flex()
        .gap_3()
        .child(file_icon)
        .child(file_details),
)
```

## Reaction

`BubbleReactions` 负责边缘定位与 reaction 区域样式。需要交互时组合现有 `Button`：

```rust
Bubble::new()
    .child("看起来没问题。")
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

组件不提供 `BubbleAction`。`Button` 已经具备 focus、键盘操作、disabled、loading 与 accessibility 行为。

## 自定义样式

两个公开 element 都实现了 `Styled`，调用方 refinement 会在默认样式之后应用：

```rust
Bubble::new()
    .rounded(cx.theme().radius)
    .bg(cx.theme().green.opacity(0.15))
    .text_color(cx.theme().green)
    .border_color(cx.theme().green.opacity(0.35))
    .child("自定义语义颜色")
```

## API 参考

- [Bubble]
- [BubbleReactions]

[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[BubbleReactions]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleReactions.html
