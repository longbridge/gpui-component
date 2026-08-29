---
title: 快速开始
description: 安装、初始化并渲染第一个 gpui-base 控件。
order: 2
---

# 快速开始

## 安装

使用与 `gpui-base` 匹配的 GPUI 仓库版本：

```toml
[dependencies]
gpui-base = { git = "https://github.com/longbridge/gpui-component" }
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

## 初始化

打开窗口前调用一次 `gpui_base::init`。若应用已调用 `gpui_component::init`，其中已经包含 Base 初始化。

```rust
use gpui::AppContext as _;

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_base::init(cx);
        // 在这里打开应用窗口。
    });
}
```

## 渲染控件并设置样式

Base 控件刻意不提供产品专属的内边距、颜色或圆角，请用普通 GPUI 方法设置样式：

```rust
use gpui::prelude::*;
use gpui::{px, rgb};
use gpui_base::Button;

Button::new("save")
    .px_3().py_2().rounded(px(6.))
    .bg(rgb(0x2563eb)).text_color(rgb(0xffffff))
    .on_click(|_, _, _| println!("save"))
    .child("Save")
```

跨渲染保持每个 `ElementId` 稳定，GPUI 才能保留元素和焦点状态。Checkbox、Switch、Radio、Toggle 等受控组件会通过回调报告下一个值；把它存进视图，并在下一次渲染时传回。

## 运行共享示例

```sh
cargo run -p gpui-base --example components -- button
```

将 `button` 替换为[原语目录](./primitives/index.md)中的 slug。网站会把同一份展示代码编译为 `wasm32-unknown-unknown` 并加载到各原语页面。
