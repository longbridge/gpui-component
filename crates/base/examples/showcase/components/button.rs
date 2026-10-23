use gpui::relative;

use super::*;

impl BaseShowcase {
    pub(in super::super) fn button(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("primary-button")
                    .px_3()
                    .h_7()
                    .line_height(relative(1.))
                    .flex()
                    .items_center()
                    .text_xs()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .bg(super::example_rgb(0x171717))
                    .text_color(super::example_rgb(0xffffff))
                    .hover(|style| style.bg(super::example_rgb(0x404040)))
                    .child("Save changes"),
            )
            .child(
                Button::new("secondary-button")
                    .px_3()
                    .h_7()
                    .line_height(relative(1.))
                    .flex()
                    .items_center()
                    .text_xs()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .bg(super::example_rgb(0xffffff))
                    .hover(|style| style.bg(super::example_rgb(0xf5f5f5)))
                    .child("Cancel"),
            )
    }
}
