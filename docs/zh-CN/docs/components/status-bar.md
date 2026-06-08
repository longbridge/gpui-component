---
title: StatusBar
description: 一个分为左、中、右三个区域的水平状态栏,通常放置在窗口或面板底部。
---

# StatusBar

StatusBar 是一个水平栏,分为 `left`、`center`、`right` 三个区域。它通常放置在窗口或面板底部,用于显示上下文信息和快捷操作。

其设计参考了原生 UI 框架中的状态栏:Windows 的 `StatusStrip`、WPF 的 `StatusBar` 以及 macOS 的 `NSStatusBar`。

## 引入

```rust
use gpui_component::status_bar::StatusBar;
```

## 区域

向区域传入任意 `impl IntoElement` —— 字符串、`Icon`、`Button`、自定义布局等。`left` 和 `right` 把项固定在两端;`child` / `children` 添加到中间区域,其对齐方式取决于固定了哪一端 —— 同时有 `left` 和 `right` 时居中,只有 `left` 时右对齐,否则左对齐(只有 `right`,或两者都没有,像普通容器一样)。多次调用即可追加更多。

- **不可交互的标签**:直接传字符串 —— 它会继承状态栏的文字样式,且没有 hover。
- **可点击的按钮**:用 `StatusBar::button(id)`,它返回一个 ghost、xsmall 的 `Button`,保证所有状态栏按钮尺寸一致。可链式调用 `label`、`icon`、`tooltip`、`on_click` 等。
- **分隔线**:用 `StatusBar::separator()`。
- **其他任意内容**:直接传该元素。

## 用法

### 标签

```rust
StatusBar::new()
    .left("Ready")
    .child("README.md")
    .right("UTF-8")
```

### 按钮

```rust
StatusBar::new()
    .left(
        StatusBar::button("branch")
            .icon(IconName::Github)
            .label("main")
            .on_click(|_, window, cx| { /* ... */ }),
    )
    .right(
        StatusBar::button("go-to-line")
            .label("Ln 1, Col 1")
            .tooltip("Go to Line/Column")
            .on_click(cx.listener(|this, _, window, cx| { /* ... */ })),
    )
```

### 分割线与自定义元素

```rust
StatusBar::new()
    .left(StatusBar::button("branch").icon(IconName::Github).label("main"))
    .left(StatusBar::divider())
    .left(
        // 任意自定义元素都可以。
        h_flex()
            .items_center()
            .gap_1()
            .child(Icon::new(IconName::CircleCheck).xsmall())
            .child("0 problems"),
    )
    .child(Progress::new("indexing").value(60.).w_24())
```

### 自定义样式

`StatusBar` 实现了 `Styled`,因此任意样式方法都会覆盖默认值。

```rust
StatusBar::new()
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .left("Ready")
```

## API 参考

### StatusBar

| 方法             | 说明                                       |
| ---------------- | ------------------------------------------ |
| `new()`          | 创建一个空的状态栏                         |
| `button(id)`     | 状态栏专用的 ghost、xsmall `Button` 预设   |
| `divider()`      | 用于分隔项的竖直分割线                     |
| `left(child)`    | 向左侧区域追加一个元素(可多次调用)       |
| `right(child)`   | 向右侧区域追加一个元素                     |
| `child(c)` / `children(cs)` | 向中间区域添加元素              |

每个区域方法接受 `impl IntoElement`。`StatusBar` 同时实现了 `Styled`,样式方法(`bg`、`border_color`、`py` 等)可以覆盖默认值。

## 注意事项

- 中间区域(通过 `child` / `children`)在同时有 `left` 和 `right` 时居中,只有 `left` 时右对齐,否则左对齐(只有 `right`,或两者都没有 —— 像普通容器一样)。
- 只读项请用纯字符串(或任意不可交互元素),以避免按钮的 hover 效果;只有可点击项才用 `StatusBar::button`。
