---
title: Bubble
description: 可承载文本、富内容和 reaction 控件的聊天消息表面。
---

# Bubble

`Bubble` 负责聊天表面及 reaction 的布局。根组件负责对齐和 80% 最大宽度，`BubbleContent` 负责可见表面的样式。

## 导入

```rust
use gpui_component::{
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide,
        BubbleReactions, BubbleVariant,
    },
    message::MessageAlignment,
};
```

## 基础用法

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(BubbleContent::new().child("可以帮我检查一下吗？"))
```

短内容可以直接作为 child 添加到 content slot：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("可以帮我检查一下吗？")
```

需要定制消息表面时使用显式 `BubbleContent`。这样 `Bubble` 上的布局 refinement 与 `BubbleContent` 上的颜色、padding、圆角和 typography 保持独立。

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
    .child("请求失败。")
```

所有 variant 都使用语义主题 token。`Ghost` 会移除边框与 content padding，并可占满整行；其他 variant 根据内容收缩，最大宽度为可用宽度的 80%。

## 富内容

任意 GPUI element 都可以作为直接 child：

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

## 分组

连续的同一发送者消息可以使用 `BubbleGroup`：

```rust
BubbleGroup::new()
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Muted)
            .child("第一条"),
    )
    .child(
        Bubble::new()
            .alignment(MessageAlignment::Start)
            .with_variant(BubbleVariant::Muted)
            .child("第二条"),
    )
```

## Reaction

`BubbleReactions` 负责边缘定位与 reaction 区域样式。需要交互时组合现有 `Button`：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .child("看起来没问题。")
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

组件不提供 `BubbleAction`。`Button` 已经具备 focus、键盘操作、disabled、loading 与 accessibility 行为。

## 自定义样式

所有公开 part 都实现了 `Styled`，调用方 refinement 会在默认样式之后应用：

```rust
Bubble::new()
    .alignment(MessageAlignment::Start)
    .content(
        BubbleContent::new()
            .rounded(cx.theme().radius)
            .bg(cx.theme().green.opacity(0.15))
            .text_color(cx.theme().green)
            .border_color(cx.theme().green.opacity(0.35))
            .child("自定义语义颜色"),
    )
```

## API 参考

- [Bubble]
- [BubbleContent]
- [BubbleGroup]
- [BubbleReactions]

[Bubble]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.Bubble.html
[BubbleContent]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleContent.html
[BubbleGroup]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleGroup.html
[BubbleReactions]: https://docs.rs/gpui-component/latest/gpui_component/bubble/struct.BubbleReactions.html
