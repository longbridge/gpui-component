---
title: Introduction
description: Rust GUI components for building fantastic cross-platform desktop application by using GPUI.
---

# GPUI Component Introduction

GPUI Component is a Rust UI component library for building fantastic desktop applications using [GPUI](https://gpui.rs).

GPUI Component is a comprehensive UI component library for building fantastic desktop applications using [GPUI](https://gpui.rs). It provides 60+ cross-platform components with modern design, theming support, and high performance.

## Features

- **Richness**: 60+ cross-platform desktop UI components
- **Native**: Inspired by macOS and Windows controls, combined with shadcn/ui design
- **Ease of Use**: Stateless `RenderOnce` components, simple and user-friendly
- **Customizable**: Built-in `Theme` and `ThemeColor`, supporting multi-theme
- **Versatile**: Supports sizes like `xs`, `sm`, `md`, and `lg`
- **Flexible Layout**: Dock layout for panel arrangements, resizing, and freeform (Tiles) layouts
- **High Performance**: Virtualized Table and List components for smooth large-data rendering
- **Content Rendering**: Native support for Markdown and simple HTML
- **Charting**: Built-in charts for visualization
- **Editor**: High performance code editor with LSP support
- **Syntax Highlighting**: Using Tree Sitter

## Quick Example

Add `gpui` and `gpui-component` to your `Cargo.toml`:

```toml
[dependencies]
gpui = "0.2"
gpui-component = "0.4.0-preview2"
```

Then create a simple "Hello, World!" application with a button:

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

## Community & Support

- [GitHub Repository](https://github.com/longbridge/gpui-component)
- [Issue Tracker](https://github.com/longbridge/gpui-component/issues)
- [Contributing Guide](https://github.com/longbridge/gpui-component/blob/main/CONTRIBUTING.md)

## License

Apache-2.0
