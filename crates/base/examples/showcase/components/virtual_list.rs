use super::*;

impl BaseShowcase {
    pub(in super::super) fn virtual_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sizes = Rc::new((0..100).map(|_| size(px(280.), px(32.))).collect());
        v_virtual_list(
            cx.entity(),
            "example-virtual-list",
            sizes,
            |_, range, _, _| {
                range
                    .map(|ix| {
                        div()
                            .h(px(28.))
                            .px_2()
                            .text_size(px(12.))
                            .flex()
                            .items_center()
                            .border_b_1()
                            .border_color(rgb(0xe5e7eb))
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .size(px(18.))
                                            .border_1()
                                            .border_color(rgb(0xa3a3a3))
                                            .child(format!("{}", (ix % 9) + 1)),
                                    )
                                    .child(format!("Customer {:03}", ix + 1)),
                            )
                            .child(format!("ID-{:04}", 1000 + ix))
                    })
                    .collect()
            },
        )
        .w(px(280.))
        .h(px(196.))
        .border_1()
        .border_color(rgb(0x171717))
    }
}
