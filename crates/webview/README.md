# Wry for GPUI

A webview supports for GPUI, based on [Wry](https://github.com/tauri-apps/wry).

This still a experimental with limited features, please file issues for any bugs or missing features.

- The WebView will render on top of the GPUI window, any GPUI elements behind the WebView bounds will be covered.
- Only supports macOS and Windows currently.

So, we recommend using the webview in a separate window or in a Popup layer.

## Run Example

In the root of the repository, run:

```
cargo run -p webview
```

## License

Apache-2.0
