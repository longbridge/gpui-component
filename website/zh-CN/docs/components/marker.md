---
title: Marker
description: 用于会话状态、通知边界和分隔标记的紧凑组合行。
---

# Marker

`Marker` 是一种轻量会话行，可用于简短状态、通知边界和时间线分隔。它接收任意 child，不定义应用专属的状态数据。

## 导入

```rust
use gpui_component::marker::{Marker, MarkerContent, MarkerIcon, MarkerVariant};
```

## 状态标记

可以直接组合现有 Icon、Spinner、Badge 或文本：

```rust
Marker::new()
    .text_color(cx.theme().green)
    .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
    .content(MarkerContent::new().child("在线"))

Marker::new()
    .icon(MarkerIcon::new().child(Spinner::new().xsmall()))
    .content(MarkerContent::new().child("Alice 正在输入…"))
```

`MarkerIcon` 提供 Base UI 的 16 px 图标 slot，`MarkerContent` 为文本或富内容提供独立样式入口。应用专属布局仍可直接添加 child。组件有意不提供 `Online`、`Typing`、`Read` 等状态 enum，这些含义和颜色由应用负责。

## 样式变体

### Plain

```rust
Marker::new()
    .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
    .content(MarkerContent::new().child("会话已归档"))
```

### Separator

```rust
Marker::new()
    .with_variant(MarkerVariant::Separator)
    .content(MarkerContent::new().child("今天"))
```

装饰线没有语义内容，因此保持为内部实现。使用 `separator_style(...)` 可以调整颜色、粗细或间距，无需增加公开的线条子组件。

### Border

```rust
Marker::new()
    .with_variant(MarkerVariant::Border)
    .content(MarkerContent::new().child("3 条未读消息"))
```

## 自定义样式

`Marker`、`MarkerIcon` 和 `MarkerContent` 都实现了 `Styled`，调用方 refinement 会在各自默认样式之后应用：

```rust
Marker::new()
    .px_3()
    .py_2()
    .rounded(cx.theme().radius)
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .child(Icon::new(IconName::Star))
    .child("已置顶消息")
```

交互 child 应使用 `Button` 或 `Link`；Marker 本身保持为非交互语义容器。

较长或禁止换行的内容可组合一个带 `min_w_0()` 样式的 `div()` child，并由调用方选择换行或截断规则。Marker 不会替任意 child 强制文本溢出策略。

## 何时不需要 Marker

Marker 是刻意保持轻量的便利组件。已有组件能够完整表达任务时，应直接使用已有组件：

- 只有数量或圆点时使用 `Badge`。
- 独立标签状态使用 `Tag`。
- 只有带文字分隔线时使用 `Separator::horizontal().label(...)`。
- 不需要统一 Marker 样式的应用专属图标文本行使用 `h_flex()`。

当这些内容需要一致的会话行表面，或需要在 plain、separator 与 border 样式之间切换时，再使用 Marker。

## API 参考

- [Marker]
- [MarkerIcon]
- [MarkerContent]

[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[MarkerIcon]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerIcon.html
[MarkerContent]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerContent.html
