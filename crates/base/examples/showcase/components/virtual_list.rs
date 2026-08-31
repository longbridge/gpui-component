use super::*;

const ITEM_COUNT: usize = 100_000;

impl BaseShowcase {
    pub(in super::super) fn virtual_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sizes = Rc::new(vec![size(px(280.), px(32.)); ITEM_COUNT]);
        div()
            .relative()
            .w_72()
            .h_48()
            .overflow_hidden()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .child(
                v_virtual_list(
                    cx.entity(),
                    "example-virtual-list",
                    sizes,
                    |_, range, _, _| {
                        range
                            .map(|ix| {
                                div()
                                    .w_full()
                                    .h_8()
                                    .px_2()
                                    .text_xs()
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(gpui::black())
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .size(px(18.))
                                                    .flex_none()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .line_height(px(18.))
                                                    .border_1()
                                                    .border_color(gpui::black())
                                                    .child(format!("{}", (ix % 9) + 1)),
                                            )
                                            .child(format!("Customer {:06}", ix + 1)),
                                    )
                                    .child(format!("ID-{:06}", 100_000 + ix))
                            })
                            .collect()
                    },
                )
                .track_scroll(&self.virtual_scroll)
                .size_full(),
            )
            .child(Scrollbar::vertical(&self.virtual_scroll).mode(ScrollbarMode::Always))
    }
}
