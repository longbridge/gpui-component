use super::*;

impl BaseShowcase {
    pub(in super::super) fn toggle_group(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let italic = self.selected_tab & 1 != 0;
        let underline = self.selected_tab & 2 != 0;
        let entity = cx.entity().downgrade();
        ToggleGroup::new("example-toggle-group")
            .flex()
            .text_size(px(13.))
            .gap_0()
            .child(self.toggle(cx))
            .child(
                Toggle::new("italic-toggle")
                    .pressed(italic)
                    .size(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
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
                                    this.selected_tab |= 1
                                } else {
                                    this.selected_tab &= !1
                                };
                                cx.notify();
                            });
                        }
                    }),
            )
            .child(
                Toggle::new("underline-toggle")
                    .pressed(underline)
                    .size(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(rgb(0x171717))
                    .when(underline, |this| {
                        this.bg(rgb(0x171717)).text_color(rgb(0xffffff))
                    })
                    .accessibility_label("Underline")
                    .child("U")
                    .on_change(move |next, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            if next {
                                this.selected_tab |= 2
                            } else {
                                this.selected_tab &= !2
                            };
                            cx.notify();
                        });
                    }),
            )
    }
}
