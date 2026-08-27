use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
    rgb,
};
use gpui_base::Radio;

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn radio(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.radio_selected == 0;
        let entity = cx.entity().downgrade();
        Radio::new("example-radio")
            .text_xs()
            .checked(checked)
            .on_change(move |next, _, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    if next {
                        this.radio_selected = 0;
                    }
                    cx.notify();
                });
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
                    .when(checked, |this| {
                        this.child(div().size(px(6.)).bg(rgb(0x171717)))
                    }),
            )
            .child(
                div().child("Standard").child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x737373))
                        .child("3–5 business days"),
                ),
            )
    }
}
