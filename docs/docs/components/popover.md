---
title: Popover
description: A floating overlay that displays rich content relative to a trigger element.
---

# Popover

Popover component for displaying floating content that appears when interacting with a trigger element. Supports multiple positioning options, custom content, different trigger methods, and automatic dismissal behaviors. Perfect for tooltips, menus, forms, and other contextual information.

## Import

```rust
use gpui_component::popover::{Popover, PopoverContent};
```

## Usage

### Basic Popover

```rust
Popover::new("basic-popover")
    .trigger(Button::new("trigger").label("Click me").outline())
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, _| {
                div()
                    .p_4()
                    .child("Hello, this is a popover!")
                    .into_any()
            })
        })
    })
```

### Popover with Custom Positioning

```rust
use gpui::Corner;

Popover::new("positioned-popover")
    .anchor(Corner::TopRight)
    .trigger(Button::new("top-right").label("Top Right").outline())
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, _| {
                div()
                    .p_4()
                    .w_64()
                    .child("This popover appears at the top right")
                    .into_any()
            })
        })
    })
```

### Form in Popover

```rust
let form = Form::new(window, cx);

Popover::new("form-popover")
    .anchor(Corner::BottomLeft)
    .trigger(Button::new("show-form").label("Open Form").outline())
    .content(move |_, _| form.clone())
```

### Right-Click Popover

```rust
use gpui::MouseButton;

Popover::new("context-menu")
    .anchor(Corner::BottomRight)
    .mouse_button(MouseButton::Right)
    .trigger(Button::new("right-click").label("Right Click Me").outline())
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                v_flex()
                    .gap_2()
                    .child("Context Menu")
                    .child(Divider::horizontal())
                    .child(
                        Button::new("action")
                            .label("Perform Action")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification("Action performed!", cx);
                                cx.emit(DismissEvent);
                            }))
                    )
                    .into_any()
            })
            .p_4()
        })
    })
```

## Advanced Usage

### Rich Content Popover

```rust
Popover::new("rich-content")
    .trigger(Button::new("info").icon(IconName::Info).outline())
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                v_flex()
                    .gap_4()
                    .max_w(px(400.))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Info).size_5())
                            .child("Information")
                            .text_lg()
                            .font_semibold()
                    )
                    .child(Divider::horizontal())
                    .child(
                        div()
                            .child("This is detailed information about the feature.")
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                Button::new("learn-more")
                                    .label("Learn More")
                                    .small()
                                    .primary()
                            )
                            .child(
                                Button::new("close")
                                    .label("Close")
                                    .small()
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        cx.emit(DismissEvent);
                                    }))
                            )
                    )
                    .into_any()
            })
            .p_4()
        })
    })
```

### Unstyled Popover

```rust
// For custom styled popovers or when you want full control
Popover::new("custom-popover")
    .appearance(false)
    .trigger(Button::new("custom").label("Custom Style"))
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                div()
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
                    .p_6()
                    .rounded_xl()
                    .shadow_2xl()
                    .child("Fully custom styled popover")
                    .into_any()
            })
        })
    })
```

### Popover with Different Triggers

```rust
// Button trigger
Popover::new("button-trigger")
    .trigger(Button::new("btn").label("Button Trigger"))
    .content(content_fn)

// Custom element trigger
Popover::new("div-trigger")
    .trigger(
        div()
            .p_2()
            .bg(cx.theme().muted)
            .rounded(px(4.))
            .child("Click this div")
            .cursor_pointer()
    )
    .content(content_fn)

// Icon trigger
Popover::new("icon-trigger")
    .trigger(Icon::new(IconName::HelpCircle).size_5())
    .content(content_fn)
```

### Dismissible Popover with Actions

```rust
Popover::new("action-popover")
    .trigger(Button::new("actions").label("Show Actions"))
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                v_flex()
                    .gap_2()
                    .child("Choose an action:")
                    .child(Divider::horizontal())
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                Button::new("copy")
                                    .label("Copy")
                                    .small()
                                    .w_full()
                                    .justify_start()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification("Copied!", cx);
                                        cx.emit(DismissEvent);
                                    }))
                            )
                            .child(
                                Button::new("paste")
                                    .label("Paste")
                                    .small()
                                    .w_full()
                                    .justify_start()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification("Pasted!", cx);
                                        cx.emit(DismissEvent);
                                    }))
                            )
                            .child(
                                Button::new("delete")
                                    .label("Delete")
                                    .small()
                                    .w_full()
                                    .justify_start()
                                    .destructive()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification("Deleted!", cx);
                                        cx.emit(DismissEvent);
                                    }))
                            )
                    )
                    .into_any()
            })
            .p_2()
            .min_w(px(120.))
        })
    })
```

### Trigger Styling

```rust
// Full width trigger
Popover::new("full-width")
    .trigger_style(StyleRefinement {
        size: Size { width: Some(relative(1.0)), ..Default::default() },
        ..Default::default()
    })
    .trigger(Button::new("full").label("Full Width Button"))
    .content(content_fn)

// Custom display
Popover::new("flex-trigger")
    .trigger_style(StyleRefinement {
        display: Some(Display::Flex),
        ..Default::default()
    })
    .trigger(Button::new("flex").label("Flex Button"))
    .content(content_fn)
```

## Positioning and Anchoring

### Anchor Positions

```rust
use gpui::Corner;

// Top left (default)
.anchor(Corner::TopLeft)    // Popover appears below trigger, aligned to left

// Top right
.anchor(Corner::TopRight)   // Popover appears below trigger, aligned to right

// Bottom left
.anchor(Corner::BottomLeft) // Popover appears above trigger, aligned to left

// Bottom right
.anchor(Corner::BottomRight) // Popover appears above trigger, aligned to right
```

### Positioning Behavior

The popover automatically:

- Snaps to window edges with 8px margin
- Adjusts position to stay within viewport
- Resolves anchor position relative to trigger bounds
- Handles collision detection with window boundaries

## Trigger Methods

### Mouse Button Configuration

```rust
use gpui::MouseButton;

// Left click (default)
.mouse_button(MouseButton::Left)

// Right click for context menus
.mouse_button(MouseButton::Right)

// Middle click
.mouse_button(MouseButton::Middle)
```

### Selectable Triggers

The trigger element must implement the `Selectable` trait. Most UI components like `Button`, `div`, etc. support this:

```rust
// Button automatically supports selection state
.trigger(Button::new("btn").label("Click me"))

// Custom elements with selection state
.trigger(my_custom_element.selected(is_selected))
```

## Custom Content

### PopoverContent Builder

The `PopoverContent` provides a flexible way to create popover content:

```rust
PopoverContent::new(window, cx, |window, cx| {
    // Return any element that implements IntoElement
    v_flex()
        .gap_3()
        .child("Content goes here")
        .child(Button::new("action").label("Action"))
        .into_any()
})
```

### Content Styling

PopoverContent can be styled using the `Styled` trait:

```rust
PopoverContent::new(window, cx, content_fn)
    .p_6()              // Custom padding
    .max_w(px(500.))    // Maximum width
    .bg(cx.theme().card) // Custom background
```

### Reusable Content Components

```rust
// Create reusable content components
struct InfoPopover {
    title: String,
    description: String,
}

impl InfoPopover {
    fn render(&self, _: &mut Window, cx: &mut Context<PopoverContent>) -> AnyElement {
        v_flex()
            .gap_3()
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .child(&self.title)
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(&self.description)
            )
            .into_any()
    }
}

// Use in popover
.content(|window, cx| {
    let info = InfoPopover {
        title: "Feature Info".to_string(),
        description: "This feature helps you...".to_string(),
    };

    cx.new(|cx| {
        PopoverContent::new(window, cx, move |window, cx| {
            info.render(window, cx)
        })
    })
})
```

### Default Styling

When not using `no_style()`, popovers automatically apply:

```rust
.bg(cx.theme().popover)                    // Background color
.text_color(cx.theme().popover_foreground) // Text color
.border_1()                                // 1px border
.border_color(cx.theme().border)           // Border color
.shadow_lg()                               // Large shadow
```

### Dismissal Events

Popovers can be dismissed by:

- Clicking outside the popover (when styled)
- Pressing the Escape key
- Emitting a `DismissEvent` from content

```rust
// Emit DismissEvent to close popover
cx.emit(DismissEvent)

// Subscribe to dismissal in content
window.subscribe(&content_view, cx, |_, _: &DismissEvent, window, cx| {
    // Handle popover dismissal
});
```

## Examples

### Tooltip-style Popover

```rust
Popover::new("tooltip")
    .trigger(
        div()
            .child("Hover me")
            .p_2()
            .cursor_help()
    )
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, _| {
                div()
                    .p_2()
                    .text_xs()
                    .child("This is helpful information")
                    .into_any()
            })
            .max_w(px(200.))
        })
    })
```

### Dropdown Menu

```rust
Popover::new("dropdown")
    .anchor(Corner::BottomLeft)
    .trigger(
        Button::new("menu")
            .label("Menu")
            .icon_after(IconName::ChevronDown)
    )
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                v_flex()
                    .gap_1()
                    .child(menu_item("New File", IconName::Plus, cx))
                    .child(menu_item("Open File", IconName::FolderOpen, cx))
                    .child(Divider::horizontal())
                    .child(menu_item("Settings", IconName::Settings, cx))
                    .into_any()
            })
            .p_1()
            .min_w(px(150.))
        })
    })

fn menu_item(label: &str, icon: IconName, cx: &Context<PopoverContent>) -> impl IntoElement {
    Button::new(label.to_lowercase().replace(" ", "-"))
        .icon(icon)
        .label(label)
        .small()
        .ghost()
        .w_full()
        .justify_start()
        .on_click(cx.listener(move |_, _, window, cx| {
            window.push_notification(format!("{} clicked", label), cx);
            cx.emit(DismissEvent);
        }))
}
```

### Confirmation Popover

```rust
Popover::new("confirm-delete")
    .anchor(Corner::TopRight)
    .trigger(
        Button::new("delete")
            .icon(IconName::Trash)
            .destructive()
    )
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::AlertTriangle).text_color(cx.theme().warning))
                            .child("Confirm Deletion")
                            .font_semibold()
                    )
                    .child(
                        div()
                            .text_sm()
                            .child("Are you sure you want to delete this item? This action cannot be undone.")
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .small()
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        cx.emit(DismissEvent);
                                    }))
                            )
                            .child(
                                Button::new("confirm")
                                    .label("Delete")
                                    .small()
                                    .destructive()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification("Item deleted", cx);
                                        cx.emit(DismissEvent);
                                    }))
                            )
                    )
                    .into_any()
            })
            .p_4()
            .max_w(px(300.))
        })
    })
```

### User Profile Popover

```rust
Popover::new("user-profile")
    .anchor(Corner::BottomRight)
    .trigger(
        div()
            .flex()
            .items_center()
            .gap_2()
            .p_2()
            .rounded(px(6.))
            .hover(|this, cx| this.bg(cx.theme().muted))
            .cursor_pointer()
            .child(Avatar::new("user").name("John Doe").size(Size::Small))
            .child("John Doe")
    )
    .content(|window, cx| {
        cx.new(|cx| {
            PopoverContent::new(window, cx, |_, cx| {
                v_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(Avatar::new("user").name("John Doe"))
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child("John Doe")
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("john.doe@example.com")
                                    )
                            )
                    )
                    .child(Divider::horizontal())
                    .child(
                        v_flex()
                            .gap_1()
                            .child(profile_menu_item("Profile", IconName::User, cx))
                            .child(profile_menu_item("Settings", IconName::Settings, cx))
                            .child(profile_menu_item("Help", IconName::HelpCircle, cx))
                            .child(Divider::horizontal())
                            .child(profile_menu_item("Sign Out", IconName::LogOut, cx))
                    )
                    .into_any()
            })
            .p_3()
            .w(px(240.))
        })
    })
```

## Performance Considerations

### Efficient Content Creation

```rust
// Good: Lazy content creation
.content(|window, cx| {
    // Content is only created when popover opens
    cx.new(|cx| expensive_content_creation(window, cx))
})

// Avoid: Pre-creating content
let content = expensive_content_creation(); // Created immediately
.content(move |_, _| content.clone())
```

### Memory Management

```rust
// The popover automatically manages content lifecycle:
// - Content is created when popover opens
// - Content is destroyed when popover closes
// - No memory leaks from unclosed popovers

// For complex content, consider cleanup:
window.subscribe(&content_view, cx, |_, _: &DismissEvent, _, _| {
    // Cleanup resources when popover closes
});
```

### Styling Performance

```rust
// Good: Use theme colors for consistency
.bg(cx.theme().popover)
.text_color(cx.theme().popover_foreground)

// Good: Minimal custom styling
PopoverContent::new(window, cx, content_fn)
    .p_4()  // Simple padding

// Avoid: Complex nested styling in hot paths
```
