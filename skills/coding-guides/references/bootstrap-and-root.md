# Bootstrap and Root Ownership

Initialize GPUI Component once, before creating component-backed views, and put `Root` at the first level of each window:

```rust
app.run(move |cx| {
    gpui_component::init(cx);

    cx.spawn(async move |cx| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            let workspace = cx.new(|cx| Workspace::new(window, cx));
            cx.new(|cx| Root::new(workspace, window, cx))
        })
        .expect("failed to open window");
    })
    .detach();
});
```

## Window-level Coordination via Root

- `Root` coordinates window-level component facilities such as overlays (dialogs, alert dialogs, sheets, notifications).
- Do not create a separate root for each page inside one window.
- `Root` coordinates modal focus restoration, focus traps, tooltip/menu layers, and window-scoped text selection.
- Bypassing `Root` can produce behavior that looks correct at rest but fails when overlays nest or focus changes quickly.
