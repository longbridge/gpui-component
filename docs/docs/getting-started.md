---
title: Getting Started
description: Learn how to set up and use GPUI Component in your project
order: -2
---

# Getting Started

## Installation

Add dependencies to your `Cargo.toml`:

```toml-vue
[dependencies]
gpui = "{{ GPUI_VERSION }}"
gpui-component = "{{ VERSION }}"
# Optional, for default bundled assets
gpui-component-assets = "{{ VERSION }}"
anyhow = "1.0"
```

:::tip
The `gpui-component-assets` crate is optional.

It provides a default set of icon assets. If you want to manage your own assets, you can skip adding this dependency.

See [Icons & Assets](./assets.md) for more details.
:::

## Quick Start

Here's a simple example to get you started:

```rust
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
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
```

:::info
Make sure to call `gpui_component::init(cx);` at first line inside the `app.run` closure. This initializes the GPUI Component system.

This is required for theming and other global settings to work correctly.
:::

## Basic Concepts

### Stateless Elements

GPUI Component uses stateless [RenderOnce] elements, making them simple and predictable. State management is handled at the view level, not in individual components.

The are all implemented [IntoElement] types.

For example:

```rs
struct MyView;

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(Button::new("btn").label("Click Me"))
            .child(Tag::secondary().child("Secondary"))
    }
}
```

### Stateful Components

There are some stateful components like `Dropdown`, `List`, and `Table` that manage their own internal state for convenience, these components implement the [Render] trait.

Those components to use are a bit different, we need create the [Entity] and hold it in the view struct.

```rs
struct MyView {
    input: Entity<InputState>,
}

impl MyView {
    fn new(window: &Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).default_value("Hello 世界"));
        Self { input }
    }
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.input.clone()
    }
}
```

### Theming

All components support theming through the built-in `Theme` system:

```rust
use gpui_component::{ActiveTheme, Theme};

// Access theme colors in your components
cx.theme().primary
cx.theme().background
cx.theme().foreground
```

### Sizing

Most components support multiple sizes:

```rust
Button::new("btn").small()
Button::new("btn").medium() // default
Button::new("btn").large()
Button::new("btn").xsmall()
```

### Variants

Components offer different visual variants:

```rust
Button::new("btn").primary()
Button::new("btn").danger()
Button::new("btn").warning()
Button::new("btn").success()
Button::new("btn").ghost()
Button::new("btn").outline()
```

## Icons

:::info
Icons are not bundled with GPUI Component to keep the library lightweight.

Continue read [Icons & Assets](./assets.md) to learn how to add icons to your project.
:::

GPUI Component has an `Icon` element, but does not include SVG files by default.

The examples use [Lucide](https://lucide.dev) icons. You can use any icons you like by naming the SVG files as defined in `IconName`. Add the icons you need to your project.

```rust
use gpui_component::{Icon, IconName};

Icon::new(IconName::Check)
Icon::new(IconName::Search).small()
```

## Next Steps

Explore the component documentation to learn more about each component:

- [Button](./components/button) - Interactive button component
- [Input](./components/input) - Text input with validation
- [Dialog](./components/dialog) - Dialog and modal windows
- [Table](./components/table) - High-performance data tables
- [More components...](./components/index)

## Development

To run the component gallery:

```bash
cargo run
```

More examples can be found in the `examples` directory:

```bash
cargo run --example <example_name>
```

[RenderOnce]: https://docs.rs/gpui/latest/gpui/trait.RenderOnce.html
[IntoElement]: https://docs.rs/gpui/latest/gpui/trait.IntoElement.html
[Render]: https://docs.rs/gpui/latest/gpui/trait.Render.html
