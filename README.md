# GPUI Component

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
- **Charting**: Built-in charts for visualizing your data.
- **Code Highlighting**: Code Editor and Syntax highlighting.
- **Wef**: (Experimental) Offscreen rendering webview based on [CEF](https://github.com/chromiumembedded/cef).

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

Check out [DEVELOPMENT.md](DEVELOPMENT.md) for more details.

## License

Apache-2.0

- UI design based on [shadcn/ui](https://ui.shadcn.com).
- Icons from [Lucide](https://lucide.dev).
