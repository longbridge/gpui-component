---
order: -7
---

# Root View

The [Root] component for as the root provider of GPUI Component features in a window. We must to use [Root] as the **first level child** of a window to enable GPUI Component features.

This is important, if we don't use [Root] as the first level child of a window, there will have some unexpected behaviors.

```rs
fn main() {
    gpui_platform::application().run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

## Overlays

We have dialogs, sheets, notifications, we need placement for them to show, so [Root] provides methods to render these overlays:

- [Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) - Render the current opened modals.
- [Root::render_sheet_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_sheet_layer) - Render the current opened drawers.
- [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) - Render the notification list.

We can put these layers in the `render` method your first level view (Root > YourFirstView):

```rs
struct MyApp;

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("My App Content")
            .children(Root::render_dialog_layer(cx))
            .children(Root::render_sheet_layer(cx))
            .children(Root::render_notification_layer(cx))
    }
}
```

:::tip
Here the example we used `children` method, it because if there is no opened dialogs, sheets, notifications, these methods will return `None`, so GPUI will not render anything.
:::

## Window Glass

We can enable the system glass effect for the window background by calling `WindowExt::set_window_glass`:

- macOS 26 (Tahoe) or later: Liquid Glass (a native `NSGlassEffectView` embedded behind the window content).
- Windows 11 22H2 or later: Mica backdrop.
- Other platforms (older systems, Linux): no-op that returns `false`, the window stays opaque.

```rs
use gpui_component::WindowExt as _;

let window = cx.open_window(options, |window, cx| {
    let view = cx.new(|_| Example);
    cx.new(|cx| Root::new(view, window, cx))
})?;

window.update(cx, |_, window, cx| {
    window.set_window_glass(true, cx);
})?;
```

Following Apple's Liquid Glass guidance (glass belongs to the navigation layer, never the content), only the navigation-layer surfaces (`sidebar`, `title_bar`, `tab_bar`) are made semi-transparent to let the glass show through them. The content area stays opaque so content is readable and never bleeds through (ghosting). The window background itself becomes transparent in glass mode, so **your content areas must paint their own opaque background** (e.g. `bg(cx.theme().background)`) to cover the glass. This applies to all windows of the application. Call `window.set_window_glass(false, cx)` to restore the opaque background.

The Story gallery has a working demo — toggle it from the title bar's window glass action.

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
