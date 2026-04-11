---
title: 上下文
description: 了解 GPUI 中的 Window 和 Context。
order: -4
---

# 上下文

在 GPUI 中，[Window]、[App]、[Context] 和 [Entity] 是最常见、也最重要的几个核心概念。

- [Window] - 当前窗口实例，负责处理 **窗口级** 行为
- [App] - 当前应用实例，负责处理 **应用级** 行为
- [Context] - 某个 Entity 的上下文实例，负责处理 **上下文级** 行为
- [Entity] - 某个实体本身，负责处理 **实体级** 状态和逻辑

例如：

```rs
fn new(window: &mut Window, cx: &mut App) {}

impl RenderOnce for MyElement {
    fn render(self, window: &mut Window, cx: &mut App) {}
}

impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
}
```

:::info
可以看到，在 GPUI 里通常都会用 `cx` 来表示 `App` 或 `Context<Self>`。

这是 GPUI 里约定俗成的命名习惯，继续沿用这个写法会让代码更统一，也更容易阅读。
:::

[Window]: https://docs.rs/gpui/latest/gpui/struct.Window.html
[App]: https://docs.rs/gpui/latest/gpui/struct.App.html
[Context]: https://docs.rs/gpui/latest/gpui/struct.Context.html
[Entity]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
