---
title: Modal
description: A modal dialog for displaying content in a layer above the app.
---

# Modal

Modal component for creating dialogs, confirmations, and alerts. Supports overlay, keyboard shortcuts, and various customizations.

## Import

```rust
use gpui_component::modal::ModalButtonProps;
use gpui_component::WindowExt;
```

## Usage

### Setup application root view for display of modals

You need to set up your application's root view to render the modal layer. This is typically done in your main application struct's render method.

The [Root::render_modal_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_modal_layer) function handles rendering any active modals on top of your app content.

```rust
use gpui_component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let modal_layer = Root::render_modal_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            // Render the modal layer on top of the app content
            .children(modal_layer)
    }
}
```

### Basic Modal

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .title("Welcome")
        .child("This is a modal dialog.")
})
```

### Form Modal

```rust
let input = cx.new(|cx| InputState::new(window, cx));

window.open_modal(cx, |modal, _, _| {
    modal
        .title("User Information")
        .child(
            v_flex()
                .gap_3()
                .child("Please enter your details:")
                .child(Input::new(&input))
        )
        .footer(|_, _, _, _| {
            vec![
                Button::new("ok")
                    .primary()
                    .label("Submit")
                    .on_click(|_, window, cx| {
                        window.close_modal(cx);
                    }),
                Button::new("cancel")
                    .label("Cancel")
                    .on_click(|_, window, cx| {
                        window.close_modal(cx);
                    }),
            ]
        })
})
```

### Confirm Modal

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .confirm()
        .child("Are you sure you want to delete this item?")
        .on_ok(|_, window, cx| {
            window.push_notification("Item deleted", cx);
            true // Return true to close modal
        })
        .on_cancel(|_, window, cx| {
            window.push_notification("Cancelled", cx);
            true
        })
})
```

### Alert Modal

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .confirm()
        .alert()
        .child("Operation completed successfully!")
        .on_close(|_, window, cx| {
            window.push_notification("Alert closed", cx);
        })
})
```

### Custom Button Labels

```rust
use gpui_component::button::ButtonVariant;

window.open_modal(cx, |modal, _, _| {
    modal
        .confirm()
        .child("Update available. Restart now?")
        .button_props(
            ModalButtonProps::default()
                .cancel_text("Later")
                .cancel_variant(ButtonVariant::Secondary)
                .ok_text("Restart Now")
                .ok_variant(ButtonVariant::Danger)
        )
        .on_ok(|_, window, cx| {
            window.push_notification("Restarting...", cx);
            true
        })
})
```

### Modal with Icon

```rust
window.open_modal(cx, |modal, _, cx| {
    modal
        .confirm()
        .child(
            h_flex()
                .gap_3()
                .child(Icon::new(IconName::TriangleAlert)
                    .size_6()
                    .text_color(cx.theme().warning))
                .child("This action cannot be undone.")
        )
})
```

### Scrollable Modal

```rust
window.open_modal(cx, |modal, window, cx| {
    modal
        .h(px(450.))
        .title("Long Content")
        .child(TextView::markdown("content", long_markdown_text, window, cx))
})
```

### Modal Options

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .title("Custom Modal")
        .overlay(true)              // Show overlay (default: true)
        .overlay_closable(true)     // Click overlay to close (default: true)
        .keyboard(true)             // ESC to close (default: true)
        .show_close(true)           // Show close button (default: true)
        .child("Modal content")
})
```

### Nested Modals

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .title("First Modal")
        .child("This is the first modal")
        .footer(|_, _, _, _| {
            vec![
                Button::new("open-another")
                    .label("Open Another Modal")
                    .on_click(|_, window, cx| {
                        window.open_modal(cx, |modal, _, _| {
                            modal
                                .title("Second Modal")
                                .child("This is nested")
                        });
                    }),
            ]
        })
})
```

### Custom Styling

```rust
window.open_modal(cx, |modal, _, cx| {
    modal
        .rounded_lg()
        .bg(cx.theme().cyan)
        .text_color(cx.theme().info_foreground)
        .title("Custom Style")
        .child("Styled modal content")
})
```

### Custom Padding

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .p_3()                      // Custom padding
        .title("Custom Padding")
        .child("Modal with custom spacing")
})
```

### Close Modal Programmatically

```rust
// From inside modal
window.close_modal(cx);

// Close and perform action
Button::new("submit")
    .primary()
    .label("Submit")
    .on_click(|_, window, cx| {
        // Do something
        window.close_modal(cx);
    })
```

## API Reference

### Modal Builder

| Method                   | Description                              |
| ------------------------ | ---------------------------------------- |
| `title(str)`             | Set modal title                          |
| `child(el)`              | Add content to modal body                |
| `footer(fn)`             | Set footer with custom buttons           |
| `overlay(bool)`          | Show/hide overlay (default: true)        |
| `overlay_closable(bool)` | Allow closing by clicking overlay        |
| `keyboard(bool)`         | Allow closing with ESC key               |
| `show_close(bool)`       | Show close button in header              |
| `confirm()`              | Use confirm modal style                  |
| `alert()`                | Use alert modal style (single OK button) |
| `button_props(props)`    | Customize confirm/alert buttons          |
| `on_ok(fn)`              | OK button callback (confirm/alert)       |
| `on_cancel(fn)`          | Cancel button callback (confirm)         |
| `on_close(fn)`           | Close callback (alert)                   |
| `min_h(px)`              | Set minimum height                       |
| `h(px)`                  | Set fixed height                         |
| `rounded_lg()`           | Apply large border radius                |
| `p_*()`                  | Custom padding                           |
| `bg()`                   | Custom background                        |

### ModalButtonProps

| Method                    | Description             |
| ------------------------- | ----------------------- |
| `ok_text(str)`            | Text for OK button      |
| `ok_variant(variant)`     | Style for OK button     |
| `cancel_text(str)`        | Text for Cancel button  |
| `cancel_variant(variant)` | Style for Cancel button |

### Window Extensions

| Method               | Description         |
| -------------------- | ------------------- |
| `open_modal(cx, fn)` | Open a modal dialog |
| `close_modal(cx)`    | Close current modal |

## Examples

### Delete Confirmation

```rust
Button::new("delete")
    .danger()
    .label("Delete")
    .on_click(|_, window, cx| {
        window.open_modal(cx, |modal, _, _| {
            modal
                .confirm()
                .child("Are you sure you want to delete this item?")
                .on_ok(|_, window, cx| {
                    // Perform delete
                    window.push_notification("Deleted", cx);
                    true
                })
        });
    })
```

### Success Alert

```rust
window.open_modal(cx, |modal, _, _| {
    modal
        .confirm()
        .alert()
        .child("Your changes have been saved successfully!")
        .on_close(|_, _, _| {
            // Optional close handler
        })
})
```
