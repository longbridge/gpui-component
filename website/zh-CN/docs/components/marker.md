---
title: Marker
description: 用于会话状态、通知边界和分隔标记的紧凑组合行。
---

# Marker

`Marker` 是一种轻量会话行，可用于简短状态、通知边界和时间线分隔。它接收任意 child，不定义应用专属的状态数据。

## 导入

```rust
use gpui_component::marker::{
    Marker, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant,
};
use gpui_component::shimmer::{ShimmerStyle, ShimmerText};
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

`MarkerIcon` 提供紧凑的标准图标 slot，`MarkerContent` 为文本或富内容提供独立样式入口。图标、间距和文字共同跟随应用的 `rem` 比例。应用专属布局仍可直接添加 child。组件有意不提供 `Online`、`Typing`、`Read` 等状态 enum，这些含义和颜色由应用负责。

## Loading 样式

启用 loading 时，Marker 的 variant、布局和普通状态外观保持不变：

```rust
Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Spinner)
    .content(MarkerContent::new().text("正在加载消息…"))

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .content(MarkerContent::new().text("正在思考…"))
```

默认 loading 样式 `Spinner` 会在没有配置 `MarkerIcon` 时自动添加紧凑的 Spinner；显式组合的图标始终优先。

`Shimmer` 会为 `MarkerContent::text(...)` 提供连续移动、自动适配主题的文字高光，可用于 ChatGPT 风格的 thinking 状态。高光基于继承的文字颜色和主题语义颜色计算，亮色、暗色主题均保持清晰的浅色光带。现有任意 `MarkerContent` child 仍然兼容，并以轻微透明度变化呈现加载状态。Icon 和分隔线保持静止；系统开启 reduced motion 时，文字完整显示且不会播放动画。

动画保留继承的文字颜色以及调用方的颜色覆盖。两种 loading 样式均可搭配任意 `MarkerVariant`，默认不启用 loading。

### 配置文字高光

使用可复用的 `ShimmerStyle` 可以调整动画表现，同时保留原有 Marker 组合方式：

```rust
use std::time::Duration;

Marker::new()
    .loading(true)
    .with_loading_style(MarkerLoadingStyle::Shimmer)
    .with_shimmer_style(
        ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .highlight_color(cx.theme().primary)
            .spread(0.45)
            .reverse(true),
    )
    .content(MarkerContent::new().text("正在处理…"))
```

默认动画周期为两秒，高光颜色跟随主题，归一化宽度为 `0.3`。`spread(...)` 接受相对于文本宽度的 `0.05..=1.0` 范围；`reverse(true)` 让高光从右向左移动，`once(true)` 让动画只播放一次。

### 在其他组件中复用

`ShimmerText` 可以独立使用，无需包裹 Marker：

```rust
ShimmerText::new("正在上传 report.pdf…")
    .with_shimmer_style(ShimmerStyle::new().spread(0.4))
    .text_sm()
    .text_color(cx.theme().muted_foreground)
```

也可以通过 `duration(...)`、`highlight_color(...)`、`spread(...)`、`reverse(...)` 和 `once(...)` 直接设置动画。`ShimmerText` 实现了 `Styled`，继承周围的文字样式和颜色，保留换行与截断行为，并在系统开启 reduced motion 时自动停止动画。

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
- [MarkerLoadingStyle]
- [MarkerIcon]
- [MarkerContent]
- [ShimmerStyle]
- [ShimmerText]

[Marker]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.Marker.html
[MarkerLoadingStyle]: https://docs.rs/gpui-component/latest/gpui_component/marker/enum.MarkerLoadingStyle.html
[MarkerIcon]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerIcon.html
[MarkerContent]: https://docs.rs/gpui-component/latest/gpui_component/marker/struct.MarkerContent.html
[ShimmerStyle]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerStyle.html
[ShimmerText]: https://docs.rs/gpui-component/latest/gpui_component/shimmer/struct.ShimmerText.html
