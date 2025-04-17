# GPUI Component

> This is still an early stage of development, we may change API frequently.
> But the features is ok to use, you must keep tracking our changes.

UI components for building fantastic desktop application by using [GPUI](https://gpui.rs).

## Features

- Accordion
- Alert
- Badge
- Breadcrumb
- Button, Link
- Checkbox, Radio, Switch
- ColorPicker
- DatePicker, DateRangePicker, Calendar
- Dock, Tiles
- Drawer
- Dropdown
- Icon
- Label
- List
- Menu
- Modal
- Notification
- Popover
- Progress & Indicator
- Resizable
- Sidebar
- Skeleton
- Slider
- Table
- Tabs
- TextInput, TextArea, OtpInput
- TextView (Markdown, Simple HTML) to native rendering, syntax highlighting.
- Theming
- TitleBar
- Toggle, ToggleGroup
- Tooltip
- WebView

## Showcase

Here is the first application: [Longbridge](https://longbridge.com) that is built by using GPUI Component.

> It still under development, not published yet.

<img width="2017" alt="SCR-20250107-kagq" src="https://github.com/user-attachments/assets/55f9e012-34ce-44d1-908f-768f8d2c8abf" />
<img width="2017" alt="SCR-20250107-kaky" src="https://github.com/user-attachments/assets/a56995ca-1c54-43bb-9a27-bc9023a169dd" />
<img width="2017" alt="SCR-20250107-kapd" src="https://github.com/user-attachments/assets/ecdfe8cd-f8d8-4df4-bafe-ab2d8517f8db" />
<img width="2017" alt="SCR-20250107-kfvk" src="https://github.com/user-attachments/assets/ccc4f25f-16c2-4140-a2ad-d194aadaa544" />

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

## Demo

If you want to see the demo, here is a some demo applications.

- [gallery-app-windows.zip](https://github.com/user-attachments/files/19796209/gallery.zip) - Updated at 2025/04/17

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
