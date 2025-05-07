# GPUI Component

UI components for building fantastic desktop application by using [GPUI](https://gpui.rs).

## Features

- **Richness**: 40+ cross-platform desktop UI components.
- **Native**: Inspired by macOS and Windows controls, combined with shadcn/ui design for a modern experience.
- **Ease of Use**: Stateless `RenderOnce` components, simple and user-friendly.
- **Customizable**: Built-in `Theme` and `ThemeColor`, supporting multi-theme and variable-based configurations.
- **Versatile**: Supports sizes like `xs`, `sm`, `md`, and `lg`.
- **Flexible Layout**: Dock layout for panel arrangements, resizing, and freeform (Tiles) layouts.
- **High Performance**: Virtualized Table and List components for smooth large-data rendering.
- **Content Rendering**: Native support for Markdown and simple HTML.

## Showcase

Here is the first application: [Longbridge Pro](https://longbridge.com/desktop) that is built by using GPUI Component.

<img width="1763" alt="Image" src="https://github.com/user-attachments/assets/3e2f4eb7-fd27-4343-b6dc-184465599e99" />

We build multi-themes support in application, this feature is not including in GPUI Component. It is based on `Theme` feature, so it easy to do.

## Usage

GPUI and GPUI Component still in development, so we need add dependency by git.

And GPUI Component depends on `gpui` by special version (It keep updated to upstream) for including WebView support.

```toml
gpui = { git = "https://github.com/huacnlee/zed.git", branch = "webview" }
gpui-component = { git = "https://github.com/longbridge/gpui-component.git" }
```

### WebView

> Still early experimental, there have a lot of limitations.

GPUI Component have `WebView` element based on [Wry](https://github.com/tauri-apps/wry), this is an optional feature, you can enable it by feature flag.

```toml
gpui-component = { git = "https://github.com/longbridge/gpui-component.git", features = ["webview"] }
```

More usage can be found in [story](https://github.com/longbridge/gpui-component/tree/main/crates/story) directory.

### Icons

GPUI Component have `Icon` element, but it does not include SVG files by default.

The example is using [Lucide](https://lucide.dev) icons, but you can use any icons you like, just named the svg files like [IconName](https://github.com/longbridge/gpui-component/blob/main/crates/ui/src/icon.rs#L86) defined the path name.
You can add icons that you need in your project.

## Development

We have a gallery of applications built with GPUI Component.

```bash
cargo run
```

More examples can be found in `examples` directory, you can run them by `cargo run --example <example_name>`.

Checkout [DEVELOPMENT](DEVELOPMENT) to see more details.

## License

Apache-2.0

- UI design based on [shadcn/ui](https://ui.shadcn.com).
- Icon from [Lucide](https://lucide.dev).
