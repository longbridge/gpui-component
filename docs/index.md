---
layout: home
---

<script setup>
import Index from './index.vue'
</script>

<Index />

## Simple and Intuitive API

Get started with just a few lines of code. Stateless components
make it easy to build complex UIs.

```rs
Button::new("ok")
    .primary()
    .label("Click Me")
    .on_click(|_, _, _| println!("Button clicked!"))
```

## Install GPUI Component

Add the following to your `Cargo.toml`:

GPUI and GPUI Component are under active development, recently GPUI have some new features not published on crates.io, so we recommend using the git version for now.

The documentation on this site are based on the **Git main branch**, if you use the crates.io version, there may be some differences.

```toml-vue
gpui = { git = "https://github.com/zed-industries/zed" }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
```

If you prefer to use the versions on crates.io. Please visit [docs.rs](https://docs.rs/gpui-component/latest/gpui_component/) to check the API differences.

```toml-vue
gpui = "{{ GPUI_VERSION }}"
gpui-component = "{{ VERSION }}"
```

## Hello World

The following `src/main.rs` is a simple "Hello, World!" application:

```rs
use gpui::*;
use gpui_component::{button::*, *};

pub struct HelloWorld;
impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
```

Run the program with the following command:

```sh
$ cargo run
```
