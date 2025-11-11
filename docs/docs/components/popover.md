---
title: Popover
description: A floating overlay that displays rich content relative to a trigger element.
---

# Popover

Popover component for displaying floating content that appears when interacting with a trigger element. Supports multiple positioning options, custom content, different trigger methods, and automatic dismissal behaviors. Perfect for tooltips, menus, forms, and other contextual information.

## Import

```rust
use gpui_component::popover::{Popover};
```

## Usage

### Basic Popover

:::info
Any element that implements [Selectable] can be used as a trigger, for example, a [Button].

Any element that implements [RenderOnce] or [Render] can be used as popover content, use `.child(...)` to add children directly.
:::

```rust
use gpui::ParentElement as _;
use gpui_component::{button::Button, popover::Popover};

Popover::new("basic-popover")
    .trigger(Button::new("trigger").label("Click me").outline())
    .child("Hello, this is a popover!")
    .child("It appears when you click the button.")
```

### Popover with Custom Positioning

The `anchor` method allows you to specify where the popover appears relative to the trigger element, this anchor point can be one of the four corners: TopLeft, TopRight, BottomLeft, BottomRight.

```rust
use gpui::Corner;

Popover::new("positioned-popover")
    .anchor(Corner::TopRight)
    .trigger(Button::new("top-right").label("Top Right").outline())
    .child("This popover appears at the top right")
```

### View in Popover

You can add any `Entity<T>` that implemented [Render] as the popover content.

```rust
let view = cx.new(|_| MyView::new());

Popover::new("form-popover")
    .anchor(Corner::BottomLeft)
    .trigger(Button::new("show-form").label("Open Form").outline())
    .child(view.clone())
```

### Right-Click Popover

Sometimes you may want to show a popover on right-click, for example, to create a special your ownen context menu. The `mouse_button` method allows you to specify which mouse button triggers the popover.

```rust
use gpui::MouseButton;

Popover::new("context-menu")
    .anchor(Corner::BottomRight)
    .mouse_button(MouseButton::Right)
    .trigger(Button::new("right-click").label("Right Click Me").outline())
    .child("Context Menu")
    .child(Divider::horizontal())
    .child("This is a custom context menu.")
```

### Dismiss Popover manually

If you want to dismiss the popover programmatically from within the content, you can emit a `DismissEvent`. In this case, you should use `content` method to create the popover content so you have access to the `cx: &mut Context<PopoverState>`.

```rust
use gpui_component::{DismissEvent, popover::Popover};

Popover::new("dismiss-popover")
    .trigger(Button::new("dismiss").label("Dismiss Popover").outline())
    .content(|_, cx| {
        div()
            .child("Click the button below to dismiss this popover.")
            .child(
                Button::new("close-btn")
                    .label("Close Popover")
                    .on_click(cx.listener(|_, _, _, cx| {
                        // NOTE: Here `cx` is `&mut Context<PopoverState>` type, so we can emit DismissEvent.
                        cx.emit(DismissEvent);
                    }))
            )
    })
```

### Styling Popover

Like the others components in GPUI Component, the `appearance(false)` method can be used to disable the default styling of the popover, allowing you to fully customize its appearance.

And the `Popover` has implemented the [Styled] trait, so you can use all the styling methods provided by GPUI to style the popover content as you like.

```rust
// For custom styled popovers or when you want full control
Popover::new("custom-popover")
    .appearance(false)
    .trigger(Button::new("custom").label("Custom Style"))
    .bg(cx.theme().accent)
    .text_color(cx.theme().accent_foreground)
    .p_6()
    .rounded_xl()
    .shadow_2xl()
    .child("Fully custom styled popover")
```

[Button]: https://docs.rs/gpui-component/latest/gpui_component/button/struct.Button.html
[Selectable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Selectable.html
[Render]: https://docs.rs/gpui/latest/gpui/trait.Render.html
[RenderOnce]: https://docs.rs/gpui/latest/gpui/trait.RenderOnce.html
[Styled]: https://docs.rs/gpui/latest/gpui/trait.Styled.html
