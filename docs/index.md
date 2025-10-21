---
layout: home
hero:
  name: GPUI Component
  text: Rust GUI components for building fantastic cross-platform desktop
    application by using GPUI.
  actions:
    - theme: brand
      text: Getting Started
      link: /docs/getting-started/
    - theme: alt
      text: GitHub
      link: https://github.com/longbridge/gpui-component
features:
  - title: 40+ Components
    details: Comprehensive library of cross-platform desktop UI components for building feature-rich applications.
  - title: High Performance
    details: Virtualized Table and List components for smooth rendering of large datasets with minimal memory footprint.
  - title: Themeable
    details: Built-in theme system with with 20+ themes, and dark mode out of the box.
  - title: Flexible Layouts
    details: Dock layout for panel arrangements, resizable panels, and freeform layouts for any application structure.
  - title: Data Visualization
    details: Built-in chart components for visualizing data with Line, Bar, Area, and Pie charts.
  - title: Code Editor
    details: High-performance code editor with LSP support, syntax highlighting, powered by Tree-sitter and Rope.
---

## Simple and Intuitive API

Get started with just a few lines of code. Stateless components
make it easy to build complex UIs.

```rs
Button::new("ok")
    .primary()
    .label("Click Me")
    .on_click(|_, _, _| println!("Button clicked!"))
```

## Install GPUI Component

Add the following to your `Cargo.toml`:

```toml
gpui-component = "0.2"
gpui = "0.2"
```

## Hello World

The following `src/main.rs` is a simple "Hello, World!" application:

```rs
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
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

Run the program with the following command:

```sh
$ cargo run
```
