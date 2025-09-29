# GPUI Component

[![Build Status](https://github.com/longbridge/gpui-component/actions/workflows/ci.yml/badge.svg)](https://github.com/longbridge/gpui-component/actions/workflows/ci.yml)

UI components for building fantastic desktop applications using [GPUI](https://gpui.rs).

## Features

- **Richness**: 40+ cross-platform desktop UI components.
- **Native**: Inspired by macOS and Windows controls, combined with shadcn/ui design for a modern experience.
- **Ease of Use**: Stateless `RenderOnce` components, simple and user-friendly.
- **Customizable**: Built-in `Theme` and `ThemeColor`, supporting multi-theme and variable-based configurations.
- **Versatile**: Supports sizes like `xs`, `sm`, `md`, and `lg`.
- **Flexible Layout**: Dock layout for panel arrangements, resizing, and freeform (Tiles) layouts.
- **High Performance**: Virtualized Table and List components for smooth large-data rendering.
- **Content Rendering**: Native support for Markdown and simple HTML.
- **Charting**: Built-in charts for visualization your data.
- **Editor**: High performance code editor (support up to 200K lines) with LSP (diagnostics, completion, hover, etc).
- **Syntax Highlighting**: Syntax highlighting for editor and markdown components using Tree Sitter.

## Showcase

Here is the first application: [Longbridge Pro](https://longbridge.com/desktop), built using GPUI Component.

<img width="1763" alt="Image" src="https://assets.lbctrl.com/uploads/32c11b27-b90d-4fce-a6b8-7d72e99fb231/longbridge-pro.png" />

We built multi-theme support in the application. This feature is not included in GPUI Component itself, but is based on the `Theme` feature, so it's easy to implement.

## Usage

GPUI and GPUI Component are still in development, so you need to add dependencies by git.

```toml
gpui = { git = "https://github.com/zed-industries/zed.git" }
gpui-component = { git = "https://github.com/longbridge/gpui-component.git" }
```

### WebView

> Still early and experimental; there are a lot of limitations.

GPUI Component has a `WebView` element based on [Wry](https://github.com/tauri-apps/wry). This is an optional feature, which you can enable with a feature flag.

```toml
gpui-component = { git = "https://github.com/longbridge/gpui-component.git", features = ["webview"] }
```

More usage examples can be found in the [story](https://github.com/longbridge/gpui-component/tree/main/crates/story) directory.

### Icons

GPUI Component has an `Icon` element, but it does not include SVG files by default.

The example uses [Lucide](https://lucide.dev) icons, but you can use any icons you like. Just name the SVG files as defined in [IconName](https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/icon.rs#L86). You can add any icons you need to your project.

## Development

We have a gallery of applications built with GPUI Component.

```bash
cargo run
```

More examples can be found in the `examples` directory. You can run them with `cargo run --example <example_name>`.

Check out [CONTRIBUTING.md](CONTRIBUTING.md) for more details.

## Compare to others

| Features              | GPUI Component                 | [Iced]      | [egui]                | [QT 6]                         |
| --------------------- | ------------------------------ | ----------- | --------------------- | ------------------------------ |
| Language              | Rust                           | Rust        | Rust                  | C++/QML                        |
| Core Render           | GPUI                           | wgpu        | wgpu                  | QT                             |
| License               | Apache 2.0                     | MIT         | MIT/Apache 2.0        | Commercial                     |
| Min Binary Size [^1]  | 12MB                           | 11MB        | 5M                    | 20MB [^2]                      |
| Cross-Platform        | Yes                            | Yes         | Yes                   | Yes                            |
| Documentation         | No                             | Simple      | Simple                | Good                           |
| Web                   | No                             | Yes         | Yes                   | Yes                            |
| UI Style              | Modern                         | Basic       | Basic                 | Basic                          |
| CJK Support           | Yes                            | Yes         | Bad                   | Yes                            |
| Chart                 | Yes                            | No          | No                    | Yes                            |
| Table (Large dataset) | Yes<br>(Virtual Rows, Columns) | No          | Yes<br>(Virtual Rows) | Yes<br>(Virtual Rows, Columns) |
| Table Column Resize   | Yes                            | No          | Yes                   | Yes                            |
| Text base             | Rope                           | String [^3] | trait TextBuffer [^4] | [QTextDocument]                |
| CodeEditor            | Simple                         | Simple      | Simple                | Basic API                      |
| Dock Layout           | Yes                            | Yes         | Yes                   | Yes                            |
| Syntax Highlight      | [Tree Sitter]                  | [Syntect]   | [Syntect]             | [QSyntaxHighlighter]           |
| Markdown Rendering    | Yes                            | Yes         | Basic                 | No                             |
| Markdown mix HTML     | Yes                            | No          | No                    | No                             |
| HTML Rendering        | Basic                          | No          | No                    | Basic                          |
| Text Selection        | TextView                       | No          | Any Label             | No                             |
| Themes                | Yes                            | No          | No                    | No                             |
| I18n                  | Yes                            | Yes         | Yes                   | Yes                            |

> Please submit an issue or PR if any mistakes or outdated are found.

[Iced]: https://github.com/iced-rs/iced
[egui]: https://github.com/emilk/egui
[QT 6]: https://www.qt.io/product/qt6
[Tree Sitter]: https://tree-sitter.github.io/tree-sitter/
[Syntect]: https://github.com/trishume/syntect
[QSyntaxHighlighter]: https://doc.qt.io/qt-6/qsyntaxhighlighter.html
[QTextDocument]: https://doc.qt.io/qt-6/qtextdocument.html

[^1]: Release builds by use simple hello world example.

[^2]: [Reducing Binary Size of Qt Applications](https://www.qt.io/blog/reducing-binary-size-of-qt-applications-part-3-more-platforms)

[^3]: Iced TextInput: https://github.com/iced-rs/iced/blob/c65bfd1afb399a74eddb3f66853f3cc385a08826/widget/src/text_input.rs#L111

[^4]: egui TextBuffer: https://github.com/emilk/egui/blob/0a81372cfd3a4deda640acdecbbaf24bf78bb6a2/crates/egui/src/widgets/text_edit/text_buffer.rs#L20

## License

Apache-2.0

- UI design based on [shadcn/ui](https://ui.shadcn.com).
- Icons from [Lucide](https://lucide.dev).
