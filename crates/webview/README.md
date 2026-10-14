# Wry for GPUI

A webview supports for GPUI, based on [Wry](https://github.com/tauri-apps/wry).

This still a experimental with limited features, please file issues for any bugs or missing features.

- On macOS with `gpui-fast`, `WebView::new` registers a native composition surface so GPUI's deferred overlays, including menus, dialogs, and tooltips, can render above the WebView. Use the wrapper's `set_bounds` and `set_visible` methods to keep the surface synchronized; raw Wry calls bypass it.
- Other backends and platforms retain the native child-view path: the WebView covers GPUI content in its rectangle. Composition initialization failures log a warning and fall back to that path.
- Only supports macOS and Windows currently.

Where native composition is unavailable, use a separate window or a layout that does not overlap GPUI overlays with the WebView.

## Run Example

In the root of the repository, run:

```
cargo run -p webview
```

## License

Apache-2.0
