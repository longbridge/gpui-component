use gpui_base::{TextView, TextViewStyle};

use super::*;

const MARKDOWN: &str = r#"# A Base-only TextView

`TextViewStyle::default()` provides readable typography, links, borders, tables, and code surfaces without an application theme.

- Select this text with the pointer.
- Copy it with the platform shortcut.
- Open [GPUI Component](https://github.com/longbridge/gpui-component).

```rust
fn main() {
    println!("syntax highlighting is opt-in");
}
```

| Capability | Default |
| --- | --- |
| Markdown and HTML | Enabled |
| Selection and copy | Enabled |
| Syntax highlighting | Disabled |
"#;

const HTML: &str = r#"
<h2>HTML uses the same renderer</h2>
<p>Common semantic markup such as <strong>strong text</strong>, <em>emphasis</em>, links, lists, code, and tables works directly in <code>gpui-base</code>.</p>
<blockquote>No gpui-component theme or tree-sitter dependency is required.</blockquote>
"#;

impl BaseShowcase {
    pub(in super::super) fn text_view(&self) -> impl IntoElement {
        div()
            .id("text-view-example")
            .debug_selector(|| "text-view-example".into())
            .w(px(720.))
            .max_w_full()
            .h(px(560.))
            .max_h_full()
            .overflow_y_scroll()
            .p_4()
            .bg(gpui::white())
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .rounded_lg()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                TextView::markdown("base-text-view-markdown", MARKDOWN)
                    .style(TextViewStyle::default())
                    .selectable(true),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(super::example_rgb(0xd4d4d4)),
            )
            .child(
                TextView::html("base-text-view-html", HTML)
                    .style(TextViewStyle::default())
                    .selectable(true),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
    use gpui_base::TextSelection;

    use crate::showcase::BaseShowcase;

    #[gpui::test]
    fn text_view_showcase_renders_with_base_defaults(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (_, window) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));

        window.update(|window, cx| window.draw(cx).clear(cx));
        let pending_frames = window.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(
            pending_frames, 0,
            "an idle TextSelectionLayer must not continuously request frames"
        );
    }

    #[gpui::test]
    fn text_view_showcase_drag_selection_settles(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (_, cx) = cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));
        let cx: &mut VisualTestContext = cx;

        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("text-view-example")
            .expect("example bounds");
        // Drag specifically across the Markdown table. A paragraph-only drag
        // does not exercise the table's multiple inline runs on each row.
        let start = point(bounds.left() + px(36.), bounds.top() + px(365.));
        let end = point(bounds.right() - px(36.), bounds.top() + px(505.));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());

        assert!(cx.update(|window, cx| TextSelection::has_selection(window, cx)));
        assert_eq!(
            cx.update(|window, cx| window.simulate_next_frame(cx)),
            0,
            "table selection must settle"
        );
    }
}
