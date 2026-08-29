use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn editor(&self) -> impl IntoElement {
        let state = self.editor.clone();
        div()
            .w_80()
            .flex()
            .flex_col()
            .items_start()
            .gap_1()
            .text_xs()
            .child(div().h(px(16.)).flex().items_center().child("Rust Editor"))
            .child(
                InputBase::new("example-editor")
                    .w_full()
                    .h_32()
                    .px_2()
                    .py_2()
                    .overflow_hidden()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .styles(|styles| {
                        styles.focused(|style| style.border_color(super::example_rgb(0x171717)))
                    })
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        state.update(cx, |state, cx| state.focus(window, cx));
                    })
                    .child(div().size_full().child(Editor::new(&self.editor))),
            )
    }
}
