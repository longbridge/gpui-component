use super::*;

impl BaseShowcase {
    pub(in super::super) fn hover_card(&self) -> impl IntoElement {
        HoverCard::new("example-hover-card")
            .trigger(
                div()
                    .id("hover-trigger")
                    .px_3()
                    .py_1()
                    .text_xs()
                    .text_color(rgb(0x171717))
                    .underline()
                    .child("Hover over gpui-base"),
            )
            .content(|_, _, _| {
                div()
                    .id("hover-content")
                    .w(px(210.))
                    .p_2()
                    .text_xs()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xd4d4d4))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size_7()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .border_1()
                                    .border_color(rgb(0x171717))
                                    .text_sm()
                                    .child("G"),
                            )
                            .child(
                                div().text_sm().child("gpui-base").child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x737373))
                                        .child("@gpui-base"),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(rgb(0x737373))
                            .child("Unstyled primitives for GPUI."),
                    )
            })
    }
}
