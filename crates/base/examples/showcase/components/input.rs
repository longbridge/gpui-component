use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn input(&self) -> impl IntoElement {
        let input_state = self.input.clone();
        let multiline_state = self.multiline_input.clone();
        div()
            .w(px(220.))
            .flex()
            .flex_col()
            .items_start()
            .gap_2()
            .text_size(px(12.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(div().h(px(16.)).flex().items_center().child("Project name"))
                    .child(
                        Input::new("example-input")
                            .w_full()
                            .h(px(28.))
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
                            .child(self.input.clone()),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_1()
                    .child(div().h(px(16.)).flex().items_center().child("Notes"))
                    .child(
                        Input::new("example-multiline-input")
                            .w_full()
                            .h(px(68.))
                            .px_2()
                            .py_2()
                            .overflow_y_scroll()
                            .border_1()
                            .border_color(rgb(0xd4d4d4))
                            .styles(|styles| {
                                styles.focused(|style| style.border_color(rgb(0x171717)))
                            })
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                multiline_state.update(cx, |state, cx| state.focus(window, cx));
                            })
                            .child(self.multiline_input.clone()),
                    ),
            )
    }
}
