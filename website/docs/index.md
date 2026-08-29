---
title: Introduction
description: A comprehensive Rust framework for building fantastic, high-performance desktop applications with GPUI.
---

# GPUI Component Introduction

GPUI Component is a comprehensive Rust desktop application framework built on [GPUI](https://gpui.rs).

It combines a complete UI system with application-grade data, layout, content,
and editing capabilities. Use `gpui-component` for polished controls with one
coherent visual language, or build your own design system on the reusable
behavior and infrastructure in `gpui-base`.

## Features

- **60+ UI Components**: Forms, navigation, overlays, feedback, layout, and more.
- **Production Ready**: Used to build Longbridge Pro from day one and refined in a publicly shipped commercial desktop application.
- **Native Feel**: Modern controls inspired by macOS and Windows.
- **120 FPS**: GPU-accelerated interfaces that remain smooth under load.
- **Data Tables**: Virtual scrolling, fixed and resizable columns, sorting, and cell selection across hundreds of thousands of rows.
- **Virtual Lists**: Render only the visible range, including differently sized items.
- **Code Editor**: 200K lines, Tree-sitter highlighting, diagnostics, completion, and hover.
- **Dock Layout**: Resizable panels, draggable tabs, nested splits, edge docks, and freeform Tiles.
- **Rich Content**: Native Markdown and HTML, syntax highlighting, and charts.
- **Design Freedom**: Use the complete visual system or build your own on `gpui-base`.
- **Typed Motion**: CSS-aligned easing, timing, keyframes, springs, presence, and measured reveal with allocation-free steady sampling.
- **Cross Platform**: Ship one Rust codebase to macOS, Windows, and Linux.

## Quick Example

Add `gpui` and `gpui-component` to your `Cargo.toml`:

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
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
    gpui_platform::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

## Community & Support

Learn how to build interruptible 120 FPS animation in the [GPUI Base Motion guide](/base/motion).

- [GitHub Repository](https://github.com/longbridge/gpui-component)
- [Issue Tracker](https://github.com/longbridge/gpui-component/issues)
- [Contributing Guide](https://github.com/longbridge/gpui-component/blob/main/CONTRIBUTING.md)

## License

Apache-2.0
