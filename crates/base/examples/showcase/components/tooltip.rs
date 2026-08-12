use super::*;

impl BaseShowcase {
    pub(in super::super) fn tooltip(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .relative()
            .gap_2()
            .text_size(px(13.))
            .child(
                Button::new("tooltip-anchor")
                    .h(px(30.))
                    .px_2()
                    .border_1()
                    .border_color(rgb(0x171717))
                    .bg(rgb(0xffffff))
                    .child("Command menu"),
            )
            .child(
                Tooltip::new("example-tooltip")
                    .px_2()
                    .py_1()
                    .text_size(px(12.))
                    .bg(rgb(0x171717))
                    .text_color(rgb(0xffffff))
                    .child("Open command menu · ⌘K"),
            )
    }
}
