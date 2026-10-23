use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _,
    relative,
};
use gpui_base::{Button, Popup};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn popup(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.popup_open;
        let entity = cx.entity().downgrade();
        Popup::new(
            "example-popup",
            Button::new("popup-trigger")
                .h_7()
                .line_height(relative(1.))
                .px_3()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::black())
                .text_color(gpui::white())
                .on_click(move |_, _, cx| {
                    _ = entity.update(cx, |this, cx| {
                        this.popup_open = !this.popup_open;
                        cx.notify();
                    });
                })
                .child(if open { "Close popup" } else { "Open popup" }),
        )
        .when(open, |this| {
            this.content(
                div()
                    .w_64()
                    .p_2()
                    .text_xs()
                    .bg(super::example_rgb(0xffffff))
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .child("Anchored surface")
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(super::example_rgb(0x737373))
                            .child("Popup positions content relative to its trigger."),
                    ),
            )
        })
    }
}
