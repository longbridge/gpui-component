use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.dialog_open;
        let entity = cx.entity().downgrade();
        let open_entity = entity.clone();

        div()
            .child(
                Button::new("open-dialog")
                    .h(px(28.))
                    .px_3()
                    .py_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(rgb(0x171717))
                    .on_click(move |_, _, cx| {
                        _ = open_entity.update(cx, |this, cx| {
                            this.dialog_open = true;
                            cx.notify();
                        });
                    })
                    .child("Edit profile"),
            )
            .child(
                Dialog::new(cx)
                    .open(open)
                    .on_open_change(move |open, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.dialog_open = open;
                            cx.notify();
                        });
                    })
                    .backdrop(
                        DialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(rgb(0x000000))
                            .opacity(0.2),
                    )
                    .popup(
                        DialogPopup::new()
                            .w(px(280.))
                            .p_3()
                            .text_size(px(12.))
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0xd4d4d4))
                            .child(
                                DialogTitle::new()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Edit profile"),
                            )
                            .child(
                                DialogDescription::new()
                                    .mt_2()
                                    .text_color(rgb(0x737373))
                                    .child("Update the public details shown on your profile."),
                            )
                            .child(div().mt_3().text_sm().child("Display name"))
                            .child(
                                Input::new("dialog-name")
                                    .mt_2()
                                    .w_full()
                                    .h(px(30.))
                                    .px_2()
                                    .border_1()
                                    .border_color(rgb(0xd4d4d4))
                                    .on_mouse_down(MouseButton::Left, {
                                        let input = self.input.clone();
                                        move |_, window, cx| {
                                            input.update(cx, |state, cx| state.focus(window, cx));
                                        }
                                    })
                                    .child(self.input.clone()),
                            )
                            .child(
                                div()
                                    .mt_3()
                                    .flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(
                                        gpui_base::DialogClose::new().child(
                                            Button::new("dialog-cancel")
                                                .h(px(28.))
                                                .px_3()
                                                .py_0()
                                                .border_1()
                                                .border_color(rgb(0xd4d4d4))
                                                .child("Cancel"),
                                        ),
                                    )
                                    .child(
                                        Button::new("dialog-save")
                                            .h(px(28.))
                                            .px_3()
                                            .py_0()
                                            .bg(rgb(0x171717))
                                            .text_color(rgb(0xffffff))
                                            .on_click({
                                                let entity = cx.entity().downgrade();
                                                move |_, _, cx| {
                                                    _ = entity.update(cx, |this, cx| {
                                                        this.dialog_open = false;
                                                        cx.notify();
                                                    });
                                                }
                                            })
                                            .child("Save changes"),
                                    ),
                            ),
                    ),
            )
    }
}
