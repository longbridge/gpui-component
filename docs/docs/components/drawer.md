---
title: Drawer
description: A sliding panel that appears from the edges of the screen for displaying content.
---

# Drawer

A Drawer (also known as a sidebar or slide-out panel) is a navigation component that slides in from the edges of the screen. It provides additional space for content without taking up the main view, and can be used for navigation menus, forms, or any supplementary content.

## Import

```rust
use gpui_component::ContextModal;
use gpui_component::Placement;
```

## Usage

### Setup application root view for display of drawers

You need to set up your application's root view to render the drawer layer. This is typically done in your main application struct's render method.

The [Root::render_drawer_layer](https://docs.rs/gpui-component/latest/gpui_component/struct.Root.html#method.render_drawer_layer) function handles rendering any active modals on top of your app content.

```rust
use gpui_component::TitleBar;

struct MyApp {
    view: AnyView,
}

impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer_layer = Root::render_drawer_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(TitleBar::new())
                    .child(div().flex_1().overflow_hidden().child(self.view.clone())),
            )
            // Render the drawer layer on top of the app content
            .children(drawer_layer)
    }
}
```

### Basic Drawer

```rust
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Navigation")
        .child("Drawer content goes here")
})
```

### Drawer with Placement

```rust
// Left drawer (default)
window.open_drawer_at(Placement::Left, cx, |drawer, _, _| {
    drawer.title("Left Drawer")
})

// Right drawer
window.open_drawer_at(Placement::Right, cx, |drawer, _, _| {
    drawer.title("Right Drawer")
})

// Top drawer
window.open_drawer_at(Placement::Top, cx, |drawer, _, _| {
    drawer.title("Top Drawer")
})

// Bottom drawer
window.open_drawer_at(Placement::Bottom, cx, |drawer, _, _| {
    drawer.title("Bottom Drawer")
})
```

### Drawer with Custom Size

```rust
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Wide Drawer")
        .size(px(500.))  // Custom width for left/right, height for top/bottom
        .child("This drawer is 500px wide")
})
```

### Drawer with Form Content

```rust
let input = cx.new(|cx| InputState::new(window, cx));
let date = cx.new(|cx| DatePickerState::new(window, cx));

window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("User Profile")
        .child(
            v_flex()
                .gap_4()
                .child("Enter your information:")
                .child(TextInput::new(&input).placeholder("Full Name"))
                .child(DatePicker::new(&date).placeholder("Date of Birth"))
        )
        .footer(
            h_flex()
                .gap_3()
                .child(Button::new("save").primary().label("Save"))
                .child(Button::new("cancel").label("Cancel"))
        )
})
```

### Overlay Options

```rust
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Settings")
        .overlay(true)              // Show overlay background (default: true)
        .overlay_closable(true)     // Click overlay to close (default: true)
        .child("Drawer settings content")
})

// No overlay
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Side Panel")
        .overlay(false)             // No overlay background
        .child("This drawer has no overlay")
})
```

### Resizable Drawer

```rust
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Resizable Panel")
        .resizable(true)            // Allow user to resize (default: true)
        .size(px(300.))
        .child("You can resize this drawer by dragging the edge")
})
```

### Custom Margin and Positioning

```rust
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Below Title Bar")
        .margin_top(px(32.))        // Space for window title bar
        .child("This drawer appears below the title bar")
})
```

### Drawer with List

```rust
let delegate = ListDelegate::new(items);
let list = cx.new(|cx| List::new(delegate, window, cx));

window.open_drawer_at(Placement::Left, cx, |drawer, _, _| {
    drawer
        .title("File Explorer")
        .size(px(400.))
        .child(
            div()
                .border_1()
                .border_color(cx.theme().border)
                .rounded(cx.theme().radius)
                .size_full()
                .child(list.clone())
        )
})
```

### Close Event Handling

```rust
window.open_drawer(cx, |drawer, _, _| {
    drawer
        .title("Drawer with Handler")
        .child("This drawer has a custom close handler")
        .on_close(|_, window, cx| {
            window.push_notification("Drawer was closed", cx);
        })
})
```

### Navigation Drawer

```rust
window.open_drawer_at(Placement::Left, cx, |drawer, _, _| {
    drawer
        .title("Navigation")
        .size(px(280.))
        .child(
            v_flex()
                .gap_2()
                .child(Button::new("home").ghost().label("Home").w_full())
                .child(Button::new("profile").ghost().label("Profile").w_full())
                .child(Button::new("settings").ghost().label("Settings").w_full())
                .child(Button::new("logout").ghost().label("Logout").w_full())
        )
})
```

### Custom Styling

```rust
window.open_drawer(cx, |drawer, _, cx| {
    drawer
        .title("Styled Drawer")
        .bg(cx.theme().accent)
        .text_color(cx.theme().accent_foreground)
        .border_color(cx.theme().primary)
        .child("Custom styled drawer content")
})
```

### Programmatic Close

```rust
// Close drawer from inside
Button::new("close")
    .label("Close Drawer")
    .on_click(|_, window, cx| {
        window.close_drawer(cx);
    })

// Close drawer from outside
window.close_drawer(cx);
```

## API Reference

### Window Extensions

| Method                              | Description                                |
| ----------------------------------- | ------------------------------------------ |
| `open_drawer(cx, fn)`               | Open drawer with default placement (Right) |
| `open_drawer_at(placement, cx, fn)` | Open drawer at specific placement          |
| `close_drawer(cx)`                  | Close current drawer                       |

### Drawer Builder

| Method                   | Description                             |
| ------------------------ | --------------------------------------- |
| `title(str)`             | Set drawer title                        |
| `child(el)`              | Add content to drawer body              |
| `footer(el)`             | Set footer content                      |
| `size(px)`               | Set drawer size (width or height)       |
| `margin_top(px)`         | Set top margin (for title bars)         |
| `resizable(bool)`        | Allow resizing (default: true)          |
| `overlay(bool)`          | Show overlay background (default: true) |
| `overlay_closable(bool)` | Click overlay to close (default: true)  |
| `on_close(fn)`           | Close event callback                    |

### Placement Options

| Value               | Description                         |
| ------------------- | ----------------------------------- |
| `Placement::Left`   | Slides in from left edge            |
| `Placement::Right`  | Slides in from right edge (default) |
| `Placement::Top`    | Slides in from top edge             |
| `Placement::Bottom` | Slides in from bottom edge          |

### Styling Methods

| Method                | Description              |
| --------------------- | ------------------------ |
| `bg(color)`           | Set background color     |
| `text_color(color)`   | Set text color           |
| `border_color(color)` | Set border color         |
| `px_*()/py_*()`       | Custom padding           |
| `gap_*()`             | Spacing between children |

## Examples

### Settings Panel

```rust
window.open_drawer_at(Placement::Right, cx, |drawer, _, _| {
    drawer
        .title("Settings")
        .size(px(350.))
        .child(
            v_flex()
                .gap_4()
                .child("Appearance")
                .child(Checkbox::new("dark-mode").label("Dark Mode"))
                .child(Checkbox::new("animations").label("Enable Animations"))
                .child("Notifications")
                .child(Checkbox::new("push-notifications").label("Push Notifications"))
        )
        .footer(
            h_flex()
                .justify_end()
                .gap_2()
                .child(Button::new("apply").primary().label("Apply"))
                .child(Button::new("cancel").label("Cancel"))
        )
})
```

### File Browser

```rust
window.open_drawer_at(Placement::Left, cx, |drawer, _, _| {
    drawer
        .title("Files")
        .size(px(300.))
        .child(
            v_flex()
                .size_full()
                .child(
                    h_flex()
                        .gap_2()
                        .p_2()
                        .child(Button::new("new-folder").small().icon(IconName::FolderPlus))
                        .child(Button::new("upload").small().icon(IconName::Upload))
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(file_tree_list)
                )
        )
})
```

### Help Panel

```rust
window.open_drawer_at(Placement::Bottom, cx, |drawer, _, _| {
    drawer
        .title("Help & Documentation")
        .size(px(200.))
        .child(
            h_flex()
                .gap_4()
                .child("Keyboard Shortcuts")
                .child(Kbd::new("⌘").child("K"))
                .child("Search")
                .child(Kbd::new("⌘").child("P"))
                .child("Command Palette")
        )
})
```

## Best Practices

1. **Appropriate Placement**: Use left/right for navigation, top/bottom for temporary content
2. **Consistent Sizing**: Maintain consistent drawer sizes across your application
3. **Clear Headers**: Always provide descriptive titles
4. **Close Options**: Provide multiple ways to close (ESC, overlay click, close button)
5. **Content Organization**: Use proper spacing and grouping for drawer content
6. **Responsive Design**: Consider drawer behavior on smaller screens
7. **Performance**: Lazy load drawer content when possible for better performance
