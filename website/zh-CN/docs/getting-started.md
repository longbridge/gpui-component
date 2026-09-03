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
gpui-kit = "0.6"
anyhow = "1.0"
```

:::tip
`gpui-kit` 始终引入 GPUI 和 `gpui-base`，并默认带上 `gpui-component` 和默认图标集。如果你希望自行管理图标与资源文件，只保留需要的 feature 即可：

```toml
gpui-kit = { version = "0.6", default-features = false, features = ["component"] }
```
更多说明见 [资源与图标](./assets.md)。
:::

## 快速开始

下面是一个最小可运行示例：

```rust
use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

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
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

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
请确保在 `app.run` 闭包中尽早调用 `gpui_kit::init(cx);`。它会初始化主题和全局配置。
:::

## 后续阅读

- [组件总览](./components/index)
- [资源与图标](./assets.md)

