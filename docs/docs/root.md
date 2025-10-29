---
order: -7
---

# Root View

The [Root] component for as the root provider of GPUI Component features in a window. We must to use [Root] as the **first level child** of a window to enable GPUI Component features.

This is important, if we don't use [Root] as the first level child of a window, there will have some unexpected behaviors.

```rs
fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Example);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

## Overlays

We have modals, drawers, notifications, we need placement for them to show, so [Root] provides methods to render these overlays:

- [Root::render_modal_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_modal_layer) - Render the current opened modals.
- [Root::render_drawer_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_drawer_layer) - Render the current opened drawers.
- [Root::render_notification_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_notification_layer) - Render the notification list.

We can put these layers in the `render` method your first level view (Root > YourFirstView):

```rs
struct MyApp;

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("My App Content")
            .children(Root::render_modal_layer(cx))
            .children(Root::render_drawer_layer(cx))
            .children(Root::render_notification_layer(cx))
    }
}
```

:::tip
Here the example we used `children` method, it because if there is no opened modals/drawers/notifications, these methods will return `None`, so GPUI will not render anything.
:::

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
