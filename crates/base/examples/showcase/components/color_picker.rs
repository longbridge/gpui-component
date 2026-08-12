use super::*;

impl BaseShowcase {
    pub(in super::super) fn color_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = [
            (0xdc2626, "#DC2626"),
            (0xd97706, "#D97706"),
            (0x16a34a, "#16A34A"),
            (0x2563eb, "#2563EB"),
            (0x7c3aed, "#7C3AED"),
        ];
        let selected = self.color_index;
        let entity = cx.entity().downgrade();
        div()
            .w_56()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size_7()
                            .bg(rgb(colors[selected].0))
                            .border_1()
                            .border_color(rgb(0x171717)),
                    )
                    .child(div().text_xs().child(colors[selected].1)),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .gap_2()
                    .children(colors.into_iter().enumerate().map(|(index, (color, _))| {
                        let entity = entity.clone();
                        div()
                            .id(format!("color-{index}"))
                            .size_7()
                            .bg(rgb(color))
                            .border_1()
                            .border_color(rgb(if index == selected {
                                0x171717
                            } else {
                                0xffffff
                            }))
                            .on_click(move |_, _, cx| {
                                _ = entity.update(cx, |this, cx| {
                                    this.color_index = index;
                                    cx.notify();
                                });
                            })
                    })),
            )
    }
}
