use super::*;

impl BaseShowcase {
    pub(in super::super) fn slider(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let percentage = self.slider.read(cx).percentage().end;
        let track_width = 220.;
        let thumb_size = 14.;
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
                        .relative()
                        .w_full()
                        .h_full()
                        .child(
                            div()
                                .absolute()
                                .top(px(13.))
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .bg(rgb(0xd4d4d4)),
                        )
                        .child(
                            SliderIndicator::new(&self.slider)
                                .absolute()
                                .inset_0()
                                .child(
                                    div()
                                        .absolute()
                                        .top(px(13.))
                                        .left_0()
                                        .w(px(track_width * percentage))
                                        .h(px(2.))
                                        .bg(rgb(0x171717)),
                                ),
                        )
                        .child(
                            SliderThumb::new(&self.slider)
                                .absolute()
                                .top(px(7.))
                                .left(px((track_width * percentage - thumb_size / 2.)
                                    .clamp(0., track_width - thumb_size)))
                                .size(px(thumb_size))
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0x171717)),
                        ),
                ),
            )
    }
}
