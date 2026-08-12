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
            .w(px(220.))
            .text_size(px(13.))
            .flex()
            .flex_col()
            .gap_2()
            .child(self.radio(cx))
            .child(
                Radio::new("express-radio")
                    .checked(!self.checked)
                    .on_change(move |next, _, _, cx| {
                        if next {
                            _ = entity.update(cx, |this, cx| {
                                this.checked = false;
                                cx.notify();
                            });
                        }
                    })
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(14.))
                            .border_1()
                            .border_color(rgb(0x171717))
                            .when(!self.checked, |this| {
                                this.child(div().size(px(6.)).bg(rgb(0x171717)))
                            }),
                    )
                    .child(
                        div().child("Express").child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(0x737373))
                                .child("Next business day"),
                        ),
                    ),
            )
            .child(
                Radio::new("pickup-radio")
                    .disabled(true)
                    .flex()
                    .items_center()
                    .gap_2()
                    .opacity(0.45)
                    .child(div().size(px(14.)).border_1().border_color(rgb(0x171717)))
                    .child(
                        div()
                            .child("Local pickup")
                            .child(div().text_size(px(12.)).child("Currently unavailable")),
                    ),
            )
    }
}
