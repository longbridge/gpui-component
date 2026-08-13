use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
    rgb,
};
use gpui_base::{Radio, RadioGroup};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn radio_group(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        RadioGroup::new("example-radio-group")
            .w_56()
            .text_xs()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.radio(cx))
            .child(
                Radio::new("express-radio")
                    .checked(self.radio_selected == 1)
                    .on_change(move |next, _, _, cx| {
                        if next {
                            _ = entity.update(cx, |this, cx| {
                                this.radio_selected = 1;
                                cx.notify();
                            });
                        }
                    })
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .mt(px(2.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(14.))
                            .border_1()
                            .border_color(rgb(0x171717))
                            .when(self.radio_selected == 1, |this| {
                                this.child(div().size(px(6.)).bg(rgb(0x171717)))
                            }),
                    )
                    .child(
                        div().child("Express").child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x737373))
                                .child("Next business day"),
                        ),
                    ),
            )
            .child(
                Radio::new("pickup-radio")
                    .disabled(true)
                    .flex()
                    .items_start()
                    .gap_2()
                    .opacity(0.45)
                    .child(
                        div()
                            .mt(px(2.))
                            .size(px(14.))
                            .border_1()
                            .border_color(rgb(0x171717)),
                    )
                    .child(
                        div()
                            .child("Local pickup")
                            .child(div().text_xs().child("Currently unavailable")),
                    ),
            )
    }
}
