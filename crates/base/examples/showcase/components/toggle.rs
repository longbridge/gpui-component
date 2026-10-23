use super::*;

impl BaseShowcase {
    pub(in super::super) fn toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pressed = self.toggle_pressed;
        let entity = cx.entity().downgrade();
        Toggle::new("example-toggle")
            .pressed(pressed)
            .on_change(move |next, _, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.toggle_pressed = next;
                    cx.notify();
                });
            })
            .size_7()
            .text_xs()
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .when(pressed, |this| {
                this.bg(super::example_rgb(0x171717))
                    .text_color(super::example_rgb(0xffffff))
            })
            .font_weight(gpui::FontWeight::BOLD)
            .accessibility_label("Bold")
            .child("B")
    }
}
