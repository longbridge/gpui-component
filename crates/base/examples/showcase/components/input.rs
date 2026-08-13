use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn input(&self) -> impl IntoElement {
        let input_state = self.input.clone();
        let textarea_state = self.textarea.clone();
        let editor_state = self.editor.clone();
        div()
            .w_56()
            .flex()
            .flex_col()
            .items_start()
            .gap_2()
            .text_xs()
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(div().h(px(16.)).flex().items_center().child("Project name"))
                    .child(
                        InputBase::new("example-input")
                            .w_full()
                            .h_7()
                            .px_2()
                            .flex()
                            .items_center()
                            .border_1()
                            .border_color(rgb(0xd4d4d4))
                            .styles(|styles| {
                                styles.focused(|style| style.border_color(rgb(0x171717)))
                            })
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                input_state.update(cx, |state, cx| state.focus(window, cx));
                            })
                            .child(Input::new(&self.input)),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(div().h(px(16.)).flex().items_center().child("Textarea"))
                    .child(
                        InputBase::new("example-multiline-input")
                            .w_full()
                            .h_16()
                            .px_2()
                            .py_2()
                            // InputState owns the multiline scroll position. Making this
                            // frame scrollable as well gives wheel events two competing
                            // coordinate spaces and breaks caret hit-testing after a scroll.
                            .overflow_hidden()
                            .border_1()
                            .border_color(rgb(0xd4d4d4))
                            .styles(|styles| {
                                styles.focused(|style| style.border_color(rgb(0x171717)))
                            })
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                textarea_state.update(cx, |state, cx| state.focus(window, cx));
                            })
                            .child(div().size_full().child(Textarea::new(&self.textarea))),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(div().h(px(16.)).flex().items_center().child("Editor"))
                    .child(
                        InputBase::new("example-editor")
                            .w_full()
                            .h_24()
                            .px_2()
                            .py_2()
                            .overflow_hidden()
                            .border_1()
                            .border_color(rgb(0xd4d4d4))
                            .styles(|styles| {
                                styles.focused(|style| style.border_color(rgb(0x171717)))
                            })
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                editor_state.update(cx, |state, cx| state.focus(window, cx));
                            })
                            .child(div().size_full().child(Editor::new(&self.editor))),
                    ),
            )
    }
}
