use gpui::{IntoElement, ParentElement as _, Styled as _, div, px, rgb};
use gpui_base::Link;

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn link(&self) -> impl IntoElement {
        div()
            .w(px(220.))
            .flex()
            .flex_col()
            .gap_2()
            .text_size(px(12.))
            .child("Navigation is application-owned")
            .child(
                Link::new("example-link")
                    .href("/base/components/link")
                    .open_with(|href, _, _, cx| cx.open_url(href))
                    .h(px(28.))
                    .px_3()
                    .py_0()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(rgb(0x171717))
                    .child("Open Link documentation  →"),
            )
            .child(
                Link::new("disabled-link")
                    .href("/disabled")
                    .disabled(true)
                    .h(px(28.))
                    .px_3()
                    .py_0()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(rgb(0xd4d4d4))
                    .text_color(rgb(0x737373))
                    .child("Disabled destination"),
            )
    }
}
