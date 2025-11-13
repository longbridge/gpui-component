---
order: -7
---

# Root View

The [Root] component for as the root provider of GPUI Component features in a window. We must to use [Root] as the **first level child** of a window to enable GPUI Component features.

This is important, if we don't use [Root] as the first level child of a window, there will have some unexpected behaviors.

```rs
struct Example;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child("Hello, World!")
            .child(Button::new("ok").child("Click Me"))
    }
}

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

Here is the layout structure:

```
Window
 └─ Root
     └─ First Child View (Example)
         └─ Div
             ├─ "Hello World"
             └─ Button "Click Me"
```

## Setup base styles

You can setup window level base styles by using `Styled` fluent method on [Root], then all child views will inherit these styles.

```rs
Root::new(view.into(), window, cx)
    // This default is `.SystemUIFont` it will use system UI font.
    .font_family("Your Special Font")
    .text_sm()
```

## Register actions

You can register window level [`Action`] by using [register_action] method on [Root],
then then actions send from entire of children views in the window will be captured and handled here.

For example, your app layout structure is like this:

```
Window
  └─ Root
      ├─ Dialog, Sheet
      └─ Your Root View
```

The `Dialog` or `Sheet` is the children of the `Root` view bulit-in design in GPUI Component, it at same level with your root view.

By GPUI's actions propagation mechanism, when you send an action from `Dialog` or `Sheet`, the action will be propagated to the `Root` view,
so you can't reiceve the action in your `Your Root View` directly.

To handle the actions, you should register the action handler in the `Root` view by using [register_action] method.

```rs
register_action(|this: &mut T, action: &YourAction, window: &mut Window, cx: &mut Context<Root>| {
    // Handle your action here.
});
```

The `T` type parameter is the your **first child view** type added in the `Root`,
we will downcast the root view to this type before call your handler.

### Example

You have a simple `HelloWorld` as the first child view in the `Root`, and you want to handle an action `MyAction` in it.

```rs
actions!(example, [ToggleSearch]);

struct Example;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child("Hello, World!")
    }
}

app.run(move |cx| {
    gpui_component::init(cx);

    // You can register your action handler here.
    cx.observe_new(|root: &mut Root, _, cx| {
        root.register_action(|this: &mut Example, _:& ToggleSearch, window, cx| {
            // Handle your action here.
            println!("ToggleSearch action received in Example view.");
        });
    }).detach();

    cx.spawn(async move |cx| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|_| Example);
            // This first level on the window, should be a Root.
            cx.new(|cx| Root::new(view, window, cx))
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .detach();
});
```

With [`cx.observe_new`](https://docs.rs/gpui/latest/gpui/struct.App.html#method.observe_new),
we can register action handler when the `Root` is created.

This is useful to let us to place the action listener beside the view and action implement definition.

## For example

We have a `list.rs` and `main.rs`.

If we want to handle an action `SelectNextItem` in the `ListView`, we can register the action handler in the `list.rs` file:

```rs
pub(super) fn init(cx: &mut App) {
    cx.observe_new(|root: &mut Root, _, cx| {
        // Register action handler for SelectNextItem action.
        root.register_action(|this: &mut HelloWorld, _: &SelectNextItem, window, cx| {
            // Handle your action here.
        })
    }).detach();
}
```

Then in the `main.rs`, we just create the window and insert the `Root` view as usual.

```rs
app.run(move |cx| {
    gpui_component::init(cx);
    list::init(cx);

    // Create the window with Root view.
});

:::warning
Please ensure the `T` type parameter is the same as the first child view type in the [`Root`],
otherwise it will panic when downcasting the view.cast Root view to target type: `T`.
:::

[Root]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html
[register_action]: https://docs.rs/gpui-component/latest/gpui_component/root/struct.Root.html#method.register_action
```
