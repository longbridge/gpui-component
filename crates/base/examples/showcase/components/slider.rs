use super::*;

impl BaseShowcase {
    pub(in super::super) fn slider(&self) -> impl IntoElement {
        div()
            .w(px(220.))
            .text_size(px(13.))
            .child(
                div()
                    .mb_2()
                    .flex()
                    .justify_between()
                    .child("Volume")
                    .child("Drag to adjust"),
            )
            .child(
                Slider::new(&self.slider).w_full().h(px(28.)).child(
                    SliderTrack::new(&self.slider)
                        .w_full()
                        .h(px(6.))
                        .mt(px(10.))
                        .border_1()
                        .border_color(rgb(0x171717))
                        .bg(rgb(0xffffff))
                        .child(
                            SliderIndicator::new(&self.slider)
                                .h_full()
                                .bg(rgb(0x171717)),
                        )
                        .child(
                            SliderThumb::new(&self.slider)
                                .size(px(16.))
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0x171717)),
                        ),
                ),
            )
    }
}
