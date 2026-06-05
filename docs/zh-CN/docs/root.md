---
order: -7
---

# Root 视图

[Root] 组件是 GPUI Component 在窗口中的根提供者。要启用 GPUI Component 的功能，必须把 [Root] 作为窗口中的 **第一层子节点**。

这一点很重要。如果不把 [Root] 放在窗口的第一层，许多行为都会出现异常或不符合预期。

```rs
fn main() {
    gpui_platform::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

## 浮层

对话框、抽屉、通知等 UI 都需要一个统一的展示层，[Root] 提供了这些浮层的渲染入口：

- [Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) - 渲染当前打开的对话框
- [Root::render_sheet_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_sheet_layer) - 渲染当前打开的抽屉
- [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) - 渲染通知列表

可以在你的第一层视图中这样放置这些图层（Root > YourFirstView）：

```rs
struct MyApp;

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("My App Content")
            .children(Root::render_dialog_layer(cx))
            .children(Root::render_sheet_layer(cx))
            .children(Root::render_notification_layer(cx))
    }
}
```

:::tip
这里使用的是 `children` 而不是 `child`，因为当没有打开的 dialog、sheet 或 notification 时，这些方法会返回 `None`，GPUI 就不会渲染任何内容。
:::

## Window Glass（窗口玻璃效果）

调用 `WindowExt::enable_window_glass` 可以为窗口背景启用系统玻璃效果：

- macOS 26（Tahoe）及以上：Liquid Glass（在窗口内容下方嵌入原生 `NSGlassEffectView`）。
- Windows 11 22H2 及以上：Mica 背景材质。
- 其它环境（更低版本系统、Linux）：不做任何事并返回 `false`，窗口保持不透明背景。

```rs
use gpui_component::WindowExt as _;

let window = cx.open_window(options, |window, cx| {
    let view = cx.new(|_| Example);
    cx.new(|cx| Root::new(view, window, cx))
})?;

window.update(cx, |_, window, cx| {
    window.enable_window_glass(cx);
})?;
```

开启后，主题中的大面积表面色（如 `background`、`title_bar`、`sidebar`）会自动变为半透明，让玻璃透出来；该变换对应用的所有窗口生效。调用 `WindowExt::disable_window_glass` 可恢复不透明背景。

完整示例参见 [window_glass example](https://github.com/longbridge/gpui-component/tree/main/examples/window_glass)。

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
