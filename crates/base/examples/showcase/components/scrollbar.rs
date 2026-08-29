use super::*;

impl BaseShowcase {
    pub(in super::super) fn scrollbar(&self) -> impl IntoElement {
        div()
            .id("example-scroll-region")
            .relative()
            .w_72()
            .h_48()
            .text_xs()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .overflow_scroll()
            .track_scroll(&self.example_scroll)
            .child(div().children((1..=20).map(|row| {
                div()
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(super::example_rgb(0xe5e7eb))
                    .justify_between()
                    .child(format!("Activity {row}"))
                    .child(if row % 3 == 0 { "Completed" } else { "Pending" })
            })))
            .child(Scrollbar::new(&self.example_scroll).mode(ScrollbarMode::Always))
    }
}
