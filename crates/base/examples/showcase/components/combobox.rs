use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn combobox(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.combobox_open;
        let query = self.combobox_query.read(cx).value().to_lowercase();
        let selected = self.combobox_selection.clone();
        let entity = cx.entity().downgrade();
        let toggle_entity = entity.clone();
        let query_state = self.combobox_query.clone();
        let trigger_query_state = self.combobox_query.clone();

        Combobox::new("example-combobox")
            .open(open)
            .on_open_change(move |open, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.combobox_open = open;
                    cx.notify();
                });
            })
            .w(px(220.))
            .child(
                div()
                    .id("combobox-trigger")
                    .w_full()
                    .h(px(30.))
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_1()
                    .border_color(rgb(0xd4d4d4))
                    .text_xs()
                    .bg(rgb(0xffffff))
                    .on_click(move |_, window, cx| {
                        let mut opening = false;
                        _ = toggle_entity.update(cx, |this, cx| {
                            this.combobox_open = !this.combobox_open;
                            opening = this.combobox_open;
                            cx.notify();
                        });
                        if opening {
                            trigger_query_state.update(cx, |state, cx| state.focus(window, cx));
                        }
                    })
                    .child(selected)
                    .child(div().text_color(rgb(0x737373)).child("⌄")),
            )
            .when(open, |combo| {
                combo.child(
                    div()
                        .mt_1()
                        .p_1()
                        .border_1()
                        .border_color(rgb(0xd4d4d4))
                        .bg(rgb(0xffffff))
                        .child(
                            Input::new("combobox-search")
                                .w_full()
                                .h(px(30.))
                                .px_2()
                                .border_1()
                                .border_color(rgb(0xe5e5e5))
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    query_state.update(cx, |state, cx| state.focus(window, cx));
                                })
                                .child(self.combobox_query.clone()),
                        )
                        .child(
                            div().mt_1().children(
                                ["GPUI", "React", "SwiftUI", "Vue"]
                                    .into_iter()
                                    .filter(|label| {
                                        query.is_empty() || label.to_lowercase().contains(&query)
                                    })
                                    .map(|label| {
                                        let entity = cx.entity().downgrade();
                                        div()
                                            .id(format!("combobox-{label}"))
                                            .px_2()
                                            .h(px(28.))
                                            .flex()
                                            .items_center()
                                            .text_xs()
                                            .hover(|s| s.bg(rgb(0xf5f5f5)))
                                            .on_click(move |_, _, cx| {
                                                _ = entity.update(cx, |this, cx| {
                                                    this.combobox_selection = label.into();
                                                    this.combobox_open = false;
                                                    cx.notify();
                                                });
                                            })
                                            .child(label)
                                    }),
                            ),
                        ),
                )
            })
    }
}
