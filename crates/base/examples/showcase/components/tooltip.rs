use super::*;

impl BaseShowcase {
    pub(in super::super) fn tooltip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.tooltip_visible;
        let entity = cx.entity().downgrade();
        let trigger = div()
            .id("tooltip-trigger")
            .on_hover(move |hovered, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.tooltip_visible = *hovered;
                    cx.notify();
                });
            })
            .child(
                Button::new("tooltip-anchor")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .bg(super::example_rgb(0xffffff))
                    .child("Command menu"),
            );

        Popup::new("example-tooltip-popup", trigger)
            .text_xs()
            .when(visible, |this| {
                this.content(
                    Tooltip::new("example-tooltip")
                        .px_2()
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(super::example_rgb(0x171717))
                        .bg(super::example_rgb(0x171717))
                        .text_color(super::example_rgb(0xffffff))
                        .child("Open command menu · ⌘K"),
                )
            })
    }
}
