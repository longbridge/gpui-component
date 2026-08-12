use super::*;

impl BaseShowcase {
    pub(in super::super) fn tooltip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = !self.checked;
        let entity = cx.entity().downgrade();

        div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.))
            .child(
                div()
                    .id("tooltip-trigger")
                    .on_hover(move |hovered, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.checked = !*hovered;
                            cx.notify();
                        });
                    })
                    .child(
                        Button::new("tooltip-anchor")
                            .h(px(30.))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(rgb(0x171717))
                            .bg(rgb(0xffffff))
                            .child("Command menu"),
                    ),
            )
            .when(visible, |this| {
                this.child(
                    Tooltip::new("example-tooltip")
                        .absolute()
                        .top(px(36.))
                        .left(px(0.))
                        .px_2()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(rgb(0x171717))
                        .bg(rgb(0x171717))
                        .text_color(rgb(0xffffff))
                        .child("Open command menu · ⌘K"),
                )
            })
    }
}
