---
title: Context
description: Learn about the Window and Context in GPUI.
order: -4
---

The [Window], [App], [Context] and [Entity] are most important things in GPUI, it appears everywhere.

- [Window] - The current window instance, which for handle the **Window Level** things.
- [App] - The current application instance, which for handle the **Application Level** things.
- [Context] - The Entity Context instance, which for handle the **Context Level** things.
- [Entity] - The Entity instance, which for handle the **Entity Level** things.

For example:

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
As you can see, we always use `cx` to represent `App` and `Context<Self>`,
which is the standard naming convention for GPUI,
we can follow this convention to make our code more readable and maintainable.
:::

[Window]: https://docs.rs/gpui/latest/gpui/struct.Window.html
[App]: https://docs.rs/gpui/latest/gpui/struct.App.html
[Context]: https://docs.rs/gpui/latest/gpui/struct.Context.html
[Entity]: https://docs.rs/gpui/latest/gpui/struct.Entity.html
