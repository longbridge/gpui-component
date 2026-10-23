use super::*;

impl BaseShowcase {
    pub(in super::super) fn collapsible(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.collapsible_open;
        let entity = cx.entity().downgrade();
        Collapsible::new()
            .open(open)
            .w_64()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().child("@gpui/base · 3 repositories"))
                    .child(
                        Button::new("collapsible-trigger")
                            .size_7()
                            .border_1()
                            .border_color(super::example_rgb(0xd4d4d4))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click(move |_, _, cx| {
                                _ = entity.update(cx, |this, cx| {
                                    this.collapsible_open = !this.collapsible_open;
                                    cx.notify();
                                });
                            })
                            .child(if open { "−" } else { "+" }),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .px_2()
                    .h_7()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .text_xs()
                    .child("gpui-component"),
            )
            .content(div().mt_2().flex().flex_col().gap_2().children(
                ["gpui-base", "gpui-storybook"].into_iter().map(|name| {
                    div()
                        .px_2()
                        .h_7()
                        .flex()
                        .items_center()
                        .border_1()
                        .border_color(super::example_rgb(0xd4d4d4))
                        .text_xs()
                        .child(name)
                }),
            ))
    }
}
