use super::*;
use gpui::{Image, ImageFormat, img};
use std::sync::Arc;

const CHECK_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m3.25 8.25 3 3 6.5-7" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;

impl BaseShowcase {
    pub(in super::super) fn checkbox(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.checkbox_checked;
        let entity = cx.entity().downgrade();
        Checkbox::new("example-checkbox")
            .checked(checked)
            .flex()
            .items_center()
            .gap_2()
            .on_change(move |state, _, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.checkbox_checked = state == CheckboxState::Checked;
                    cx.notify();
                });
            })
            .child(
                CheckboxIndicator::new()
                    .checked(checked)
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_4()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .when(checked, |this| {
                        this.bg(super::example_rgb(0x171717)).child(
                            img(Arc::new(Image::from_bytes(
                                ImageFormat::Svg,
                                CHECK_SVG.to_vec(),
                            )))
                            .size(px(12.)),
                        )
                    }),
            )
            .child(div().text_xs().child("Enable product updates"))
    }
}
