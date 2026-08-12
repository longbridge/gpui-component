use super::*;

impl BaseShowcase {
    pub(in super::super) fn toast(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = !self.checked;
        let entity = cx.entity().downgrade();
        div()
            .w(px(280.))
            .h(px(158.))
            .text_size(px(13.))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Button::new("show-toast")
                    .h(px(30.))
                    .px_2()
                    .border_1()
                    .border_color(rgb(0x171717))
                    .bg(rgb(0xffffff))
                    .child("Save changes")
                    .on_click({
                        let show_entity = entity.clone();
                        move |_, _, cx| {
                            _ = show_entity.update(cx, |this, cx| {
                                this.checked = false;
                                cx.notify();
                            });
                        }
                    }),
            )
            .when(visible, |this| {
                this.child(
                    Toast::new("example-toast")
                        .transition_status(ToastTransitionStatus::Present)
                        .absolute()
                        .right_0()
                        .bottom_0()
                        .w(px(260.))
                        .p_2()
                        .border_1()
                        .border_color(rgb(0x171717))
                        .bg(rgb(0xffffff))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Changes saved"),
                                )
                                .child(Button::new("dismiss-toast").px_1().child("×").on_click({
                                    let entity = entity.clone();
                                    move |_, _, cx| {
                                        _ = entity.update(cx, |this, cx| {
                                            this.checked = true;
                                            cx.notify();
                                        });
                                    }
                                })),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_color(rgb(0x737373))
                                .child("Your preferences are now up to date."),
                        ),
                )
            })
    }
}
