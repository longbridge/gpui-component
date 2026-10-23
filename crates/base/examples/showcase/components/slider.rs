use super::*;
use gpui::relative;

impl BaseShowcase {
    pub(in super::super) fn slider(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let percentage = self.slider.read(cx).percentage().end;
        let thumb_size = 14.;
        div()
            .w_56()
            .text_xs()
            .child(
                div()
                    .mb_2()
                    .flex()
                    .justify_between()
                    .child("Volume")
                    .child("Drag to adjust"),
            )
            .child(
                Slider::new(&self.slider).w_full().h_7().child(
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
                                .bg(super::example_rgb(0xd4d4d4)),
                        )
                        .child(
                            SliderIndicator::new(&self.slider)
                                .absolute()
                                .top(px(13.))
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .left_0()
                                        .right(relative(1. - percentage))
                                        .bg(super::example_rgb(0x171717)),
                                ),
                        )
                        .child(
                            SliderThumb::new(&self.slider)
                                .absolute()
                                .top(px(7.))
                                .left(relative(percentage))
                                .ml(px(-thumb_size / 2.))
                                .size(px(thumb_size))
                                .bg(super::example_rgb(0xffffff))
                                .border_1()
                                .border_color(super::example_rgb(0x171717)),
                        ),
                ),
            )
    }
}
