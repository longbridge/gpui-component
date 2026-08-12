use gpui::{Context, IntoElement, ParentElement as _, Styled as _, div, px, rgb};
use gpui_base::OtpInput;

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn otp_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let value: Vec<char> = self.otp.read(cx).value().chars().collect();
        let active = value.len().min(5);

        div()
            .w(px(220.))
            .flex()
            .flex_col()
            .gap_1()
            .text_size(px(12.))
            .child(div().text_xs().child("Verification code"))
            .child(
                div().child(
                    OtpInput::new(&self.otp)
                        .flex()
                        .gap_1()
                        .children((0..6).map(|ix| {
                            div()
                                .size(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .border_1()
                                .border_color(if ix == active {
                                    rgb(0x171717)
                                } else {
                                    rgb(0xd4d4d4)
                                })
                                .child(value.get(ix).copied().unwrap_or(' ').to_string())
                        })),
                ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x737373))
                    .child("Enter the 6-digit code."),
            )
    }
}
