---
name: gpui-layout-and-style
description: Describes how GPUI implements layout and styling.
---

## Overview

GPUI's styling system provides a flexible way to define the appearance and layout of UI elements. It uses a flexbox-based layout engine (powered by Taffy) and supports Tailwind CSS-like style methods for convenient styling.

## Core Concepts

### Elements and Styling

All UI elements in GPUI implement the `IntoElement` trait, allowing them to be styled using method chaining:

```rust
div()
    .flex()
    .justify_center()
    .items_center()
    .bg(rgb(0x00ff00))
    .size_full()
    .child("Hello World")
```

> In GPUI style, we perfer use `gpui::Hsla` to as the color type.

### Layout Engine

GPUI uses Taffy, a high-performance flexbox layout library, for positioning and sizing elements, 
they are based on Tailwind, most methods are similar to Tailwind CSS utility classes.

- **Flexbox Properties**: `flex()`, `flex_row()`, `flex_col()`, `justify_*()`, `items_*()`, etc.
- **Sizing**: `size()`, `size_full()`, `w()`, `h()`, `min_w()`, `max_w()`, etc.
- **Spacing**: `gap()`, `m()`, `p()`, `mx()`, `my()`, etc.
- **Positioning**: `absolute()`, `relative()`, `top()`, `left()`, etc.

### Style System

Styles are applied through method chaining on elements:

```rust
struct MyComponent {
    count: usize,
}

impl Render for MyComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .justify_between()
            .items_center()
            .p_4()
            .rounded_md()
            .bg(cx.theme().colors().surface_background)
            .border_1()
            .border_color(cx.theme().colors().border)
            .child(format!("Count: {}", self.count))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("increment")
                            .label("+")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count += 1;
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("decrement")
                            .label("-")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.count = this.count.saturating_sub(1);
                                cx.notify();
                            }))
                    )
            )
    }
}
```

### Add child elements

Use `child` to add child elements, and `children` to add multiple children (If vec is empty do nothing):

```rust
use gpui::ParentElement as _;

div()
    .flex()
    .flex_col()
    .gap_4()
    .child("Child 0")
    .children(vec![
        div().child("Child 1"),
        div().child("Child 2"),
        div().child("Child 3"),
    ])
```

### Conditional

> Need use `gpui::prelude::FluentBuilder`.

Use `.when()` and `.when_some()` for conditional styling:

```rust
use gpui::prelude::FluentBuilder as _;

div()
    .flex()
    .when(self.is_active, |el| el.bg(rgb(0x00ff00)))
    .when_some(self.custom_color, |el, color| el.bg(color))
    .child("Conditional styling")
```

Or we can use `map` method to have more complex conditional logic:

```rust
use gpui::prelude::FluentBuilder as _;

div()
    .flex()
    .map(|el| {
        if self.is_active {
            el.bg(rgb(0x00ff00))
        } else {
            el.bg(rgb(0xff0000))
        }
    })
    .child("Conditional styling")
```

### Layout Context

Layout is computed in multiple phases:
1. **Request Layout**: Elements declare their size requirements
2. **Prepaint**: Elements prepare for painting with computed bounds
3. **Paint**: Elements render themselves within their assigned bounds

Elements receive their computed bounds during the prepaint phase and must respect these bounds during painting.

### Performance Considerations

- Styles are cached when possible
- Avoid creating new styles in render methods
- Use `SharedString` for text content to avoid allocations
- Prefer conditional styling methods over runtime style computation

### Common Patterns

1. **Flexbox Layouts**:
   ```rust
   div()
       .flex()
       .flex_col()
       .justify_center()
       .items_center()
   ```

2. **Grid Layouts** (using flexbox):
   ```rust
   div()
       .flex()
       .flex_wrap()
       .gap_4()
   ```

3. **Absolute Positioning**:
   ```rust
   div()
       .absolute()
       .top_4()
       .right_4()
   ```

4. **Scrollable Content**:
   ```rust
   div()
       .overflow_y_scroll()
       .max_h(px(300.0))
   ```

### Debugging Layout

Use GPUI's inspector to debug layout issues:
- Enable with `cx.open_inspector()`
- Inspect element bounds and styles
- Check for layout conflicts

Remember that all styling in GPUI is immutable - each method returns a new styled element rather than modifying the existing one.</content>
<parameter name="filePath">.claude/skills/style/SKILL.md
