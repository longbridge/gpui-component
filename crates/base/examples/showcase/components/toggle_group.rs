use super::*;

impl BaseShowcase {
    pub(in super::super) fn toggle_group(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let italic = self.toggle_group_selection & 1 != 0;
        let underline = self.toggle_group_selection & 2 != 0;
        let entity = cx.entity().downgrade();
        ToggleGroup::new("example-toggle-group")
            .flex()
            .text_xs()
            .gap_0()
            .child(self.toggle(cx))
            .child(
                Toggle::new("italic-toggle")
                    .pressed(italic)
                    .size_7()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_l_0()
                    .border_color(rgb(0x171717))
                    .when(italic, |this| {
                        this.bg(rgb(0x171717)).text_color(rgb(0xffffff))
                    })
                    .accessibility_label("Italic")
                    .child("I")
                    .on_change({
                        let entity = entity.clone();
                        move |next, _, _, cx| {
                            _ = entity.update(cx, |this, cx| {
                                if next {
                                    this.toggle_group_selection |= 1
                                } else {
                                    this.toggle_group_selection &= !1
                                };
                                cx.notify();
                            });
                        }
                    }),
            )
            .child(
                Toggle::new("underline-toggle")
                    .pressed(underline)
                    .size_7()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_l_0()
                    .border_color(rgb(0x171717))
                    .when(underline, |this| {
                        this.bg(rgb(0x171717)).text_color(rgb(0xffffff))
                    })
                    .accessibility_label("Underline")
                    .child("U")
                    .on_change(move |next, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            if next {
                                this.toggle_group_selection |= 2
                            } else {
                                this.toggle_group_selection &= !2
                            };
                            cx.notify();
                        });
                    }),
            )
    }
}
