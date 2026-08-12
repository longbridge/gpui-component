use super::*;

impl BaseShowcase {
    pub(in super::super) fn toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pressed = self.checked;
        let entity = cx.entity().downgrade();
        Toggle::new("example-toggle")
            .pressed(pressed)
            .on_change(move |next, _, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.checked = next;
                    cx.notify();
                });
            })
            .size(px(30.))
            .text_size(px(13.))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(rgb(0x171717))
            .when(pressed, |this| {
                this.bg(rgb(0x171717)).text_color(rgb(0xffffff))
            })
            .font_weight(gpui::FontWeight::BOLD)
            .accessibility_label("Bold")
            .child("B")
    }
}
