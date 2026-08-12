---
title: Getting Started
description: Install, initialize, and render your first gpui-base control.
order: 2
---

# Getting Started

## Install

Use the repository revision of GPUI that matches `gpui-base`:

```toml
[dependencies]
gpui-base = { git = "https://github.com/longbridge/gpui-component" }
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

## Initialize

Call `gpui_base::init` once before opening windows. If the application already calls `gpui_component::init`, base initialization is included.

```rust
use gpui::AppContext as _;

fn main() {
    gpui_platform::application().run(|cx| {
        gpui_base::init(cx);
        // Open your application window here.
    });
}
```

## Render and style a control

Base controls intentionally have no product-specific padding, colors, or radius. Style them with ordinary GPUI methods:

```rust
use gpui::prelude::*;
use gpui::{px, rgb};
use gpui_base::Button;

Button::new("save")
    .px_3()
    .py_2()
    .rounded(px(6.))
    .bg(rgb(0x2563eb))
    .text_color(rgb(0xffffff))
    .on_click(|_, _, _| println!("save"))
    .child("Save")
```

Keep each `ElementId` stable across renders so GPUI can preserve element and focus state. Controlled components such as Checkbox, Switch, Radio, and Toggle report the next value through callbacks; store that value in your view and pass it back on the next render.

## Run the shared examples

The examples used by this website also run as a native GPUI application:

```sh
cargo run -p gpui-base --example base_components -- button
```

Replace `button` with a component slug from the [component catalog](./components/index.md). The website compiles the same showcase for `wasm32-unknown-unknown` and loads it on each component page.
