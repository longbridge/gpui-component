use super::*;

impl BaseShowcase {
    pub(in super::super) fn scrollbar(&self) -> impl IntoElement {
        div()
            .id("example-scroll-region")
            .relative()
            .w(px(280.))
            .h(px(188.))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0x171717))
            .overflow_scroll()
            .track_scroll(&self.scroll)
            .child(div().children((1..=20).map(|row| {
                div()
                    .h(px(28.))
                    .px_2()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(0xe5e7eb))
                    .justify_between()
                    .child(format!("Activity {row}"))
                    .child(if row % 3 == 0 { "Completed" } else { "Pending" })
            })))
            .child(Scrollbar::new(&self.scroll))
    }
}
