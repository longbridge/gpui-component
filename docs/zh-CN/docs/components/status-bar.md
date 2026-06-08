---
title: StatusBar
description: 一个分为左、中、右三个区域的水平状态栏,通常放置在窗口或面板底部。
---

# StatusBar

StatusBar 是一个水平栏,分为 `left`、`center`、`right` 三个区域,使用 `justify_between` 布局。它通常放置在窗口或面板底部,用于显示上下文信息和快捷操作。

其设计参考了原生 UI 框架中的状态栏:Windows 的 `StatusStrip`、WPF 的 `StatusBar` 以及 macOS 的 `NSStatusBar`。每个区域都可接受任意元素,配套的 `StatusBarItem` 则覆盖了「图标 + 文本 + 点击」这一常见模式。

## 引入

```rust
use gpui_component::status_bar::{StatusBar, StatusBarItem};
```

## 用法

### 左右两区

```rust
StatusBar::new()
    .left(StatusBarItem::new("status").label("Ready"))
    .right(StatusBarItem::new("encoding").label("UTF-8"))
```

### 三个区域

多次调用 `left`、`center` 或 `right` 可以向同一区域追加更多项。

```rust
StatusBar::new()
    .left(StatusBarItem::new("branch").icon(IconName::Github).label("main"))
    .center(StatusBarItem::new("title").label("README.md"))
    .right(StatusBarItem::new("position").label("Ln 1, Col 1"))
```

### 带图标的项

```rust
StatusBar::new()
    .left(StatusBarItem::new("info").icon(IconName::Info).label("12 issues"))
    .right(StatusBarItem::new("lang").icon(IconName::Globe).label("Rust"))
```

### 可点击项

设置 `on_click` 后,点击该项会触发处理函数。

```rust
StatusBar::new()
    .left(
        StatusBarItem::new("go-to-line")
            .icon(IconName::CircleCheck)
            .label("Ln 1, Col 1")
            .tooltip("Go to Line/Column")
            .on_click(cx.listener(|this, _, window, cx| {
                // 处理点击
            })),
    )
    .right(StatusBarItem::new("encoding").label("UTF-8"))
```

### 自定义样式

`StatusBar` 实现了 `Styled`,因此任意样式方法都会覆盖默认值。

```rust
StatusBar::new()
    .bg(cx.theme().secondary)
    .border_color(cx.theme().border)
    .left(StatusBarItem::new("status").label("Ready"))
```

## API 参考

### StatusBar

| 方法             | 说明                                       |
| ---------------- | ------------------------------------------ |
| `new()`          | 创建一个空的状态栏                         |
| `left(child)`    | 向左侧区域追加一个子元素(可多次调用)     |
| `center(child)`  | 向中间区域追加一个子元素                   |
| `right(child)`   | 向右侧区域追加一个子元素                   |

`StatusBar` 还实现了 `Styled`,样式方法(`bg`、`border_color`、`py` 等)可以覆盖默认值。

### StatusBarItem

`StatusBarItem` 会渲染成一个 ghost `xsmall` `Button`,因此尺寸和样式与同一状态栏中的按钮保持一致。

| 方法              | 说明                       |
| ----------------- | -------------------------- |
| `new(id)`         | 使用给定的 id 创建一个项    |
| `icon(icon)`      | 设置前置图标               |
| `label(text)`     | 设置标签文本               |
| `tooltip(text)`   | 设置悬停时显示的提示       |
| `on_click(fn)`    | 设置点击处理函数           |

## 注意事项

- 三个区域使用 `justify_between` 分布;即使 `center` 为空,`left` 和 `right` 仍会固定在两端。
- `StatusBar` 设置了 `flex_shrink_0`,因此放在 flex 列底部、与 `flex_1` 内容区相邻时,它的高度不会被压缩。
- 各区域可接受任意元素,因此你可以在 `StatusBarItem` 之外放置 `Button`、`Progress` 等组件。
