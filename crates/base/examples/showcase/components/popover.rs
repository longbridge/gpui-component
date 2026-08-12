use gpui::{InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, div, px, rgb};
use gpui_base::{Button, Popover};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn popover(&self) -> impl IntoElement {
        Popover::new("example-popover")
            .trigger(
                Button::new("popover-trigger")
                    .h(px(28.))
                    .px_3()
                    .py_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(0x171717))
                    .text_color(rgb(0xffffff))
                    .child("Open popover"),
            )
            .content(|_, _, cx| {
                let state = cx.entity().downgrade();
                div()
                    .id("popover-content")
                    .w(px(240.))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_size(px(12.))
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xd4d4d4))
                    .child("Workspace access")
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x737373))
                            .child("Anyone with the link can view."),
                    )
                    .child(
                        div().mt_1().flex().justify_end().child(
                            Button::new("popover-done")
                                .h(px(26.))
                                .px_3()
                                .py_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .border_1()
                                .border_color(rgb(0x171717))
                                .on_click(move |_, window, cx| {
                                    _ = state.update(cx, |state, cx| state.dismiss(window, cx));
                                })
                                .child("Done"),
                        ),
                    )
            })
    }
}
