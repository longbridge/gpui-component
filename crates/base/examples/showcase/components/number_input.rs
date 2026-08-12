use gpui::{Context, IntoElement, ParentElement as _, Styled as _, div, px, rgb};
use gpui_base::NumberInput;

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn number_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let valid = self.input.read(cx).value().parse::<f64>().is_ok();

        div()
            .w(px(200.))
            .flex()
            .flex_col()
            .gap_1()
            .text_size(px(12.))
            .child(div().text_xs().child("Quantity"))
            .child(
                NumberInput::new(&self.input)
                    .controls_right()
                    .w_full()
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(if valid { rgb(0x171717) } else { rgb(0x737373) })
                    .input(div().w_full().px_2().child(self.input.clone()))
                    .decrement_button(|button| {
                        button
                            .w(px(24.))
                            .flex_1()
                            .h_full()
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_l_1()
                            .border_t_1()
                            .border_color(rgb(0xd4d4d4))
                            .child(div().w(px(8.)).h(px(1.)).bg(rgb(0x171717)))
                    })
                    .increment_button(|button| {
                        button
                            .w(px(24.))
                            .flex_1()
                            .min_h_0()
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_l_1()
                            .border_color(rgb(0xd4d4d4))
                            .child(
                                div()
                                    .relative()
                                    .size(px(8.))
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(3.5))
                                            .left_0()
                                            .w(px(8.))
                                            .h(px(1.))
                                            .bg(rgb(0x171717)),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left(px(3.5))
                                            .w(px(1.))
                                            .h(px(8.))
                                            .bg(rgb(0x171717)),
                                    ),
                            )
                    }),
            )
            .child(div().text_xs().text_color(rgb(0x737373)).child(if valid {
                "Step: 1"
            } else {
                "Enter a number"
            }))
    }
}
