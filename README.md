<p align="center">
  <img src="https://raw.githubusercontent.com/longbridge/gpui-component/main/website/public/logo.svg" width="112" alt="GPUI Component logo" />
  <br>
  <strong>GPUI Component</strong>
</p>

[English](./README.md) | [简体中文](./README.zh-CN.md)

[![Build Status](https://github.com/longbridge/gpui-component/actions/workflows/ci.yml/badge.svg)](https://github.com/longbridge/gpui-component/actions/workflows/ci.yml) [![Docs](https://docs.rs/gpui-component/badge.svg)](https://docs.rs/gpui-component/) [![Crates.io](https://img.shields.io/crates/v/gpui-component.svg)](https://crates.io/crates/gpui-component)

Build fantastic, high-performance desktop apps with Rust and [GPUI](https://gpui.rs).

GPUI Component is a comprehensive Rust desktop application framework. It
combines a production-ready UI system with application-grade data, layout, and
editing capabilities, all built on a reusable foundation of behavior, state,
and infrastructure.

## Features

- **60+ UI Components**: Forms, navigation, overlays, feedback, layout, and more, with polished interactions and productive defaults.
- **Production Ready**: Used to build Longbridge Pro from day one and continuously refined in a publicly shipped commercial desktop application.
- **Native Feel**: Modern controls inspired by macOS and Windows, backed by semantic themes and multiple sizes.
- **120 FPS**: GPU-accelerated interfaces that remain smooth under load.
- **Data Tables**: Virtual scrolling, fixed and resizable columns, sorting, and cell selection across hundreds of thousands of rows.
- **Virtual Lists**: Render only the visible range, including lists whose items have different sizes.
- **Code Editor**: Stable performance at 200K lines with Tree-sitter highlighting and LSP diagnostics, completion, and hover.
- **Dock Layout**: Resizable panels, draggable tabs, nested splits, edge docks, and serializable freeform Tiles.
- **Rich Content**: Native Markdown and HTML rendering, syntax highlighting, and built-in charts.
- **Design Freedom**: Use the complete visual system or build your own on the behavior and infrastructure in `gpui-base`.
- **Cross Platform**: Ship one Rust codebase to macOS, Windows, and Linux.

## Framework Architecture

### Two layers. One ecosystem.

Use `gpui-component` to keep the application coherent with one complete visual
and interaction system. Use `gpui-base` when your product needs to create and
own that system itself.

| **`gpui-component`**             | **`gpui-base`**                               |
| -------------------------------- | --------------------------------------------- |
| Complete, styled components      | Unstyled behavior and infrastructure          |
| Productive defaults with theming | Full control over structure and visual design |
| Best for building applications   | Best for building design systems              |

```text
                             APPLICATION
                                  │
                ┌─────────────────┴─────────────────┐
                │                                   │
                ▼                                   ▼
       ┌──────────────────┐               ┌──────────────────┐
       │  gpui-component  │               │ Your Design      │
       │    Styled UI     │               │ System           │
       └────────┬─────────┘               └────────┬─────────┘
                │                                  │
                └────────────────┬─────────────────┘
                                 ▼
                       ┌──────────────────┐
                       │    gpui-base     │
                       │ Behavior · State │
                       │ Infrastructure   │
                       └────────┬─────────┘
                                ▼
                              GPUI
```

> **Behavior belongs to the foundation. Presentation belongs to the application.**

Use **`gpui-component`** when you want polished controls ready to ship. Build on
**`gpui-base`** when your application should own its component source, layout,
styling, and motion while reusing difficult interaction behavior.

The layering follows the same separation that makes the
[shadcn](https://ui.shadcn.com) ecosystem flexible:

| GPUI Component ecosystem             | Web ecosystem                   |
| ------------------------------------ | ------------------------------- |
| [GPUI](https://gpui.rs)              | HTML + Tailwind CSS             |
| [`gpui-base`](crates/base/README.md) | [Base UI](https://base-ui.com)  |
| `gpui-component`                     | shadcn's styled component layer |

[Explore the architecture →](docs/ARCHITECTURE.md)

## Showcase

GPUI Component has powered [Longbridge Pro](https://longbridge.com/desktop)
from day one. The framework is extracted from the demands of a publicly shipped
commercial desktop application rather than designed in isolation.

> **GPUI provides the rendering foundation. Longbridge provides the production foundation.**

<img width="1763" alt="Image" src="https://github.com/user-attachments/assets/e1ecb9c3-2dd3-431e-bd97-5a819c30e551" />

## Usage

```toml
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
```

### Basic Example

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

### Icons

GPUI Component has an `Icon` element, but it does not include SVG files by default.

The example uses [Lucide](https://lucide.dev) icons, but you can use any icons you like. Just name the SVG files as defined in [IconName](https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/icon.rs#L86). You can add any icons you need to your project.

## Skills for AI Coding Agents

Install the GPUI Component skills for your AI coding agent (Cursor, Claude Code, Gemini CLI, Codex, etc.):

```bash
npx skills add longbridge/gpui-component
```

| Skill | Description |
| --- | --- |
| `gpui-component` | Component catalog, usage patterns, and contributor code style guide. |
| `gpui` | Low-level GPUI framework mechanics (elements, entities, async, focus, actions, tests). |

## Development

### Desktop Gallery (Story)

The `story` crate is a gallery application that showcases all available components. Run it with:

```bash
cargo run
```

### Examples

Some important examples are built into the `story` crate and can be run directly:

```bash
# Code editor with LSP support and syntax highlighting
cargo run --example editor

# Dock layout system (panels, split views, tabs)
cargo run --example dock

# Markdown rendering
cargo run --example markdown

# HTML rendering
cargo run --example html
```

The `examples` directory also contains standalone examples, each focused on a single feature. Each example is a separate crate, run them with `cargo run -p <name>`:

```bash
# Basic hello world
cargo run -p hello_world

# System monitor (real-time charts with CPU/memory data)
cargo run -p system_monitor

# Window title customization
cargo run -p window_title
```

Check out [CONTRIBUTING.md](CONTRIBUTING.md) for more details.

## Compare to others

| Features              | GPUI Component                 | [Iced]             | [egui]                | [Qt 6]                                            |
| --------------------- | ------------------------------ | ------------------ | --------------------- | ------------------------------------------------- |
| Language              | Rust                           | Rust               | Rust                  | C++/QML                                           |
| Core Render           | GPUI                           | wgpu               | wgpu                  | QT                                                |
| License               | Apache 2.0                     | MIT                | MIT/Apache 2.0        | [Commercial/LGPL](https://www.qt.io/qt-licensing) |
| Min Binary Size [^1]  | 12MB                           | 11MB               | 5M                    | 20MB [^2]                                         |
| Cross-Platform        | Yes                            | Yes                | Yes                   | Yes                                               |
| Documentation         | Simple                         | Simple             | Simple                | Good                                              |
| Web                   | Yes (WASM)                     | Yes                | Yes                   | Yes                                               |
| UI Style              | Modern                         | Basic              | Basic                 | Basic                                             |
| CJK Support           | Yes                            | Yes                | Bad                   | Yes                                               |
| Chart                 | Yes                            | No                 | No                    | Yes                                               |
| Table (Large dataset) | Yes<br>(Virtual Rows, Columns) | No                 | Yes<br>(Virtual Rows) | Yes<br>(Virtual Rows, Columns)                    |
| Table Column Resize   | Yes                            | No                 | Yes                   | Yes                                               |
| Text base             | Rope                           | [COSMIC Text] [^3] | trait TextBuffer [^4] | [QTextDocument]                                   |
| CodeEditor            | Simple                         | Simple             | Simple                | Basic API                                         |
| Dock Layout           | Yes                            | Yes                | Yes                   | Yes                                               |
| Syntax Highlight      | [Tree Sitter]                  | [Syntect]          | [Syntect]             | [QSyntaxHighlighter]                              |
| Markdown Rendering    | Yes                            | Yes                | Basic                 | No                                                |
| Markdown mix HTML     | Yes                            | No                 | No                    | No                                                |
| HTML Rendering        | Basic                          | No                 | No                    | Basic                                             |
| Text Selection        | TextView                       | No                 | Any Label             | Yes                                               |
| Custom Theme          | Yes                            | Yes                | Yes                   | Yes                                               |
| Built Themes          | Yes                            | No                 | No                    | No                                                |
| I18n                  | Yes                            | Yes                | Yes                   | Yes                                               |

> Please submit an issue or PR if any mistakes or outdated are found.

[Iced]: https://github.com/iced-rs/iced
[egui]: https://github.com/emilk/egui
[QT 6]: https://www.qt.io/product/qt6
[Tree Sitter]: https://tree-sitter.github.io/tree-sitter/
[Syntect]: https://github.com/trishume/syntect
[QSyntaxHighlighter]: https://doc.qt.io/qt-6/qsyntaxhighlighter.html
[QTextDocument]: https://doc.qt.io/qt-6/qtextdocument.html
[COSMIC Text]: https://github.com/pop-os/cosmic-text

[^1]: Release builds by use simple hello world example.

[^2]: [Reducing Binary Size of Qt Applications](https://www.qt.io/blog/reducing-binary-size-of-qt-applications-part-3-more-platforms)

[^3]: Iced Editor: <https://github.com/iced-rs/iced/blob/db5a1f6353b9f8520c4f9633d1cdc90242c2afe1/graphics/src/text/editor.rs#L65-L68>

[^4]: egui TextBuffer: <https://github.com/emilk/egui/blob/0a81372cfd3a4deda640acdecbbaf24bf78bb6a2/crates/egui/src/widgets/text_edit/text_buffer.rs#L20>

## License

Apache-2.0

- UI design based on [shadcn/ui](https://ui.shadcn.com), some from [Reui](https://reui.io).
- Icons from [Lucide](https://lucide.dev).
