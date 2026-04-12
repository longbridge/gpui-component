---
title: 开始使用
description: 学习如何在项目中安装并使用 GPUI Component。
order: -2
---

# 开始使用

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed" }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
# 可选：使用内置默认资源
gpui-component-assets = { git = "https://github.com/longbridge/gpui-component" }
anyhow = "1.0"
```

:::tip
`gpui-component-assets` 是可选依赖。

如果你希望自行管理图标与资源文件，可以不添加它。更多说明见 [资源与图标](./assets.md)。
:::

## 快速开始

下面是一个最小可运行示例：

```rust
use gpui::*;
use gpui_component::{button::*, *};

pub struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

:::info
请确保在 `app.run` 闭包中尽早调用 `gpui_component::init(cx);`。它会初始化主题和全局配置。
:::

## 后续阅读

- [组件总览](./components/index)
- [资源与图标](./assets.md)


