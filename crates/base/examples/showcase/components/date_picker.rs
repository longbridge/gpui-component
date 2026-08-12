use super::*;

impl BaseShowcase {
    pub(in super::super) fn date_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.date_open;
        let entity = cx.entity().downgrade();
        let toggle = entity.clone();
        DatePicker::new("example-date-picker", &self.date_focus)
            .open(open)
            .on_open_change(move |open, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.date_open = open;
                    cx.notify();
                });
            })
            .relative()
            .w(px(220.))
            .text_size(px(12.))
            .child(
                Button::new("date-trigger")
                    .w_full()
                    .h(px(30.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_1()
                    .border_color(rgb(0xa3a3a3))
                    .bg(rgb(0xffffff))
                    .on_click(move |_, _, cx| {
                        _ = toggle.update(cx, |this, cx| {
                            this.date_open = !this.date_open;
                            cx.notify();
                        });
                    })
                    .child("Aug 12, 2026")
                    .child("⌄"),
            )
            .when(open, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(px(34.))
                        .left_0()
                        .w_full()
                        .bg(rgb(0xffffff))
                        .child(self.calendar()),
                )
            })
    }
}
