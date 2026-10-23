use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement as _, Styled as _, div, px,
    relative,
};
use gpui_base::{Button, NumberInput};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn number_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let valid = self.input.read(cx).value().parse::<f64>().is_ok();

        fn render_btn(this: Button, icon: AnyElement) -> Button {
            this.w(px(24.))
                .flex_1()
                .min_h_0()
                .line_height(relative(1.))
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::black())
                .text_color(gpui::white())
                .hover(|this| this.bg(gpui::black().opacity(0.8)))
                .child(icon)
        }

        fn minus_icon() -> AnyElement {
            div()
                .w(px(8.))
                .h(px(1.))
                .bg(gpui::white())
                .into_any_element()
        }

        fn plus_icon() -> AnyElement {
            div()
                .relative()
                .size(px(8.))
                .child(
                    div()
                        .absolute()
                        .top(px(3.5))
                        .left_0()
                        .w_full()
                        .h(px(1.))
                        .bg(gpui::white()),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(3.5))
                        .top_0()
                        .h_full()
                        .w(px(1.))
                        .bg(gpui::white()),
                )
                .into_any_element()
        }

        div()
            .w(px(200.))
            .flex()
            .flex_col()
            .gap_1()
            .text_xs()
            .child(div().text_xs().child("Quantity"))
            .child(
                NumberInput::new(&self.input)
                    .controls_right()
                    .w_full()
                    .h_7()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(if valid {
                        super::example_rgb(0x171717)
                    } else {
                        super::example_rgb(0x737373)
                    })
                    .input(div().w_full().px_2().child(self.input.clone()))
                    .decrement_button(|button| render_btn(button, minus_icon()))
                    .increment_button(|button| render_btn(button, plus_icon())),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(super::example_rgb(0x737373))
                    .child(if valid { "Step: 1" } else { "Enter a number" }),
            )
    }
}
