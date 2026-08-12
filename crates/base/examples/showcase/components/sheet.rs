use super::*;

impl BaseShowcase {
    pub(in super::super) fn sheet(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = !self.checked;
        let entity = cx.entity().downgrade();
        let open_sheet = entity.clone();
        let trigger = Button::new("open-sheet")
            .h(px(30.))
            .px_2()
            .text_size(px(13.))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(rgb(0x171717))
            .bg(rgb(0xffffff))
            .child("Open settings")
            .on_click(move |_, _, cx| {
                _ = open_sheet.update(cx, |this, cx| {
                    this.checked = false;
                    cx.notify();
                });
            });

        div()
            .relative()
            .size_full()
            .min_h(px(260.))
            .text_size(px(13.))
            .flex()
            .items_center()
            .justify_center()
            .child(trigger)
            .when(open, |this| {
                this.child(
                    Sheet::new(cx)
                        .absolute()
                        .inset_0()
                        .request_close({
                            let entity = entity.clone();
                            move |_, cx| {
                                _ = entity.update(cx, |this, cx| {
                                    this.checked = true;
                                    cx.notify();
                                });
                            }
                        })
                        .overlay(div().absolute().inset_0().bg(rgb(0x000000)).opacity(0.15))
                        .surface(
                            div()
                                .absolute()
                                .right_0()
                                .top_0()
                                .h_full()
                                .w(px(210.))
                                .p_3()
                                .bg(rgb(0xffffff))
                                .border_l_1()
                                .border_color(rgb(0x171717))
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Settings"),
                                )
                                .child(
                                    div().mt_4().child("Workspace name").child(
                                        div()
                                            .mt_1()
                                            .h(px(30.))
                                            .px_2()
                                            .flex()
                                            .items_center()
                                            .border_1()
                                            .border_color(rgb(0xa3a3a3))
                                            .child("Acme Studio"),
                                    ),
                                )
                                .child(
                                    div()
                                        .mt_2()
                                        .text_color(rgb(0x525252))
                                        .child("Update the workspace preferences for your team."),
                                )
                                .child(
                                    div()
                                        .mt_4()
                                        .py_1()
                                        .border_t_1()
                                        .border_color(rgb(0xd4d4d4))
                                        .child("Notifications  ·  Enabled"),
                                )
                                .child(
                                    div().mt_3().flex().justify_end().child(
                                        Button::new("close-sheet")
                                            .h(px(28.))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .border_1()
                                            .border_color(rgb(0x171717))
                                            .child("Done")
                                            .on_click({
                                                let entity = entity.clone();
                                                move |_, _, cx| {
                                                    _ = entity.update(cx, |this, cx| {
                                                        this.checked = true;
                                                        cx.notify();
                                                    });
                                                }
                                            }),
                                    ),
                                ),
                        ),
                )
            })
    }
}
