---
title: Dialog
description: A dialog dialog for displaying content in a layer above the app.
---

# Dialog

Dialog component for creating dialogs, confirmations, and alerts. Supports overlay, keyboard shortcuts, and various customizations.

## Import

```rust
use gpui_component::dialog::DialogButtonProps;
use gpui_component::WindowExt;
```

## Usage

### Setup application root view for display of dialogs

You need to set up your application's root view to render the dialog layer. This is typically done in your main application struct's render method.

The [Root::render_dialog_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_dialog_layer) function handles rendering any active dialogs on top of your app content.

```rust
use gpui_component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            // Render the dialog layer on top of the app content
            .children(dialog_layer)
    }
}
```

### Basic Dialog

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Welcome")
        .child("This is a dialog dialog.")
})
```

### Form Dialog

```rust
let input = cx.new(|cx| InputState::new(window, cx));

window.open_dialog(cx, |dialog, _, _| {
    dialog
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
                        window.close_dialog(cx);
                    }),
                Button::new("cancel")
                    .label("Cancel")
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
            ]
        })
})
```

### Confirm Dialog

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .confirm()
        .child("Are you sure you want to delete this item?")
        .on_ok(|_, window, cx| {
            window.push_notification("Item deleted", cx);
            true // Return true to close dialog
        })
        .on_cancel(|_, window, cx| {
            window.push_notification("Cancelled", cx);
            true
        })
})
```

### Alert Dialog

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
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

window.open_dialog(cx, |dialog, _, _| {
    dialog
        .confirm()
        .child("Update available. Restart now?")
        .button_props(
            DialogButtonProps::default()
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

### Dialog with Icon

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
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

### Scrollable Dialog

```rust
use gpui_component::text::markdown;

window.open_dialog(cx, |dialog, window, cx| {
    dialog
        .h(px(450.))
        .title("Long Content")
        .child(markdown(long_markdown_text))
})
```

### Dialog Options

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("Custom Dialog")
        .overlay(true)              // Show overlay (default: true)
        .overlay_closable(true)     // Click overlay to close (default: true)
        .keyboard(true)             // ESC to close (default: true)
        .close_button(false)        // Show close button (default: true)
        .child("Dialog content")
})
```

### Nested Dialogs

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .title("First Dialog")
        .child("This is the first dialog")
        .footer(|_, _, _, _| {
            vec![
                Button::new("open-another")
                    .label("Open Another Dialog")
                    .on_click(|_, window, cx| {
                        window.open_dialog(cx, |dialog, _, _| {
                            dialog
                                .title("Second Dialog")
                                .child("This is nested")
                        });
                    }),
            ]
        })
})
```

### Custom Styling

```rust
window.open_dialog(cx, |dialog, _, cx| {
    dialog
        .rounded_lg()
        .bg(cx.theme().cyan)
        .text_color(cx.theme().info_foreground)
        .title("Custom Style")
        .child("Styled dialog content")
})
```

### Custom Padding

```rust
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .p_3()                      // Custom padding
        .title("Custom Padding")
        .child("Dialog with custom spacing")
})
```

### Close Dialog Programmatically

The `close_dialog` method can be used to close the active dialog from anywhere within the window context.

```rust
// Close top level active dialog.
window.close_dialog(cx);

// Close and perform action
Button::new("submit")
    .primary()
    .label("Submit")
    .on_click(|_, window, cx| {
        // Do something
        window.close_dialog(cx);
    })
```

## Examples

### Delete Confirmation

```rust
Button::new("delete")
    .danger()
    .label("Delete")
    .on_click(|_, window, cx| {
        window.open_dialog(cx, |dialog, _, _| {
            dialog
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
window.open_dialog(cx, |dialog, _, _| {
    dialog
        .confirm()
        .alert()
        .child("Your changes have been saved successfully!")
        .on_close(|_, _, _| {
            // Optional close handler
        })
})
```
