use super::*;

impl BaseShowcase {
    pub(in super::super) fn switch(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.switch_checked;
        let entity = cx.entity().downgrade();
        div()
            .w_64()
            .text_xs()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div().child("Automatic updates").child(
                    div()
                        .mt_1()
                        .text_xs()
                        .text_color(rgb(0x737373))
                        .child("Install stable releases automatically."),
                ),
            )
            .child(
                Switch::new("example-switch")
                    .checked(checked)
                    .on_change(move |next, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.switch_checked = next;
                            cx.notify();
                        });
                    })
                    .child(
                        SwitchTrack::new("example-switch-track")
                            .checked(checked)
                            .w(px(36.))
                            .h(px(20.))
                            .p(px(2.))
                            .bg(if checked {
                                rgb(0x171717)
                            } else {
                                rgb(0xd4d4d4)
                            })
                            .child(
                                SwitchThumb::new(checked)
                                    .size_4()
                                    .bg(rgb(0xffffff))
                                    .ml(if checked { px(16.) } else { px(0.) }),
                            ),
                    ),
            )
    }
}
