# Wry for GPUI

A WebView implementation for GPUI, based on [Wry](https://github.com/tauri-apps/wry).

This is still experimental with limited features. Please file issues for bugs or missing features.

- On macOS, GPUI Popovers, Dialogs, Notifications, and Popup Menus can render
  above the WebView when using the layered-scene GPUI branch configured by
  this repository.
- The WebView owns native keyboard focus after it is clicked. `gpui-wry`
  clears GPUI's logical focus at that boundary so an Input does not continue
  showing a stale blinking caret.
- On macOS, applications must provide standard Edit-menu actions for Cut,
  Copy, Paste, and Select All. AppKit routes their keyboard shortcuts through
  the responder chain to either the focused WebView or GPUI.
- Only macOS and Windows are currently supported. Windows overlay composition
  remains future work.

See [`WEBVIEW_OVERLAY_RESEARCH.md`](WEBVIEW_OVERLAY_RESEARCH.md) for the
composition, focus, input, and platform details.

## Run Example

In the root of the repository, run:

```
cargo run -p webview
```

## License

Apache-2.0
