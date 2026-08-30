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
            .w(px(720.))
            .max_w_full()
            .h(px(560.))
            .max_h_full()
            .overflow_y_scroll()
            .p_4()
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
    use gpui::TestAppContext;

    use crate::showcase::BaseShowcase;

    #[gpui::test]
    fn text_view_showcase_renders_with_base_defaults(cx: &mut TestAppContext) {
        cx.update(gpui_base::init);
        let (_, window) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-view", window, cx));

        window.update(|window, cx| window.draw(cx).clear(cx));
    }
}
