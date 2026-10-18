# GPUI WebView

A webview supports for GPUI, based on [Wry](https://github.com/tauri-apps/wry).

This crate was previously published as `gpui-wry`. Replace the dependency with `gpui-webview` and `gpui_wry::` paths with `gpui_webview::`.

This still a experimental with limited features, please file issues for any bugs or missing features.

Supported platforms:

| Platform | Engine | GPUI overlays above the page |
| --- | --- | --- |
| macOS | WKWebView | With `gpui-fast` |
| Windows | WebView2 | Not yet; the WebView covers overlays |
| Linux (X11 / XWayland) | WebKitGTK | With `gpui-fast` |
| Linux (Wayland) | — | Not supported; run on XWayland |

To render GPUI overlays above the page on macOS and Linux, enable the `gpui-fast` feature:

```toml
[dependencies]
gpui-webview = { version = "0.7", features = ["gpui-fast"] }
```

- The crate uses the default gpui-pre backend unless its `gpui-fast` feature is enabled. That feature also enables `gpui-fast` on `gpui-kit`, so the whole application runs on GPUI Fast. If something else in the dependency graph enables `gpui-kit/gpui-fast` without it, the crate fails to compile with an error that asks for `gpui-webview/gpui-fast`.
- On gpui-pre, the native view is attached to the window directly. It covers GPUI content in its bounds, including popovers, dialogs, menus and tooltips. On macOS and Linux, a view that its container clips only partly is hidden until it is fully visible again.
- With `gpui-fast` on macOS, `WebView::new()` registers the native view with window composition so GPUI overlays can render above it. Composition failures are logged and fall back to the plain native attachment.
- With `gpui-fast` on Linux, `WebView::build()` builds the native view in a window composition surface, with the same fallback.
- Use `WebView::set_bounds()` and `WebView::set_visible()` to keep the managed container and Wry child synchronized. Raw Wry handles bypass this coordination.
- Use `WebView::build()` to attach a webview to a GPUI window with the platform setup Wry needs.
- On Linux, the webview uses WebKitGTK and requires the application to run on X11 (or XWayland in a Wayland session). Start it with `gpui_kit::platform::linux(WindowingModes::X11)`; `WebView::build()` returns an error on a Wayland window. Wayland is not supported.

On Windows, or without `gpui-fast`, use the webview in a separate window or a Popup layer when overlapping GPUI content is required.

## Run Example

In the root of the repository, run:

```
cargo run -p webview
```

## License

Apache-2.0
