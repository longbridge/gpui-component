use super::super::color_field::{ColorFieldEvent, ColorFieldState};
use super::super::color_slider::sizing as slider_constants;
use super::super::color_slider::{ColorSliderEvent, ColorSliderState};
use super::super::color_spec::Hsv;
use super::super::delegates::ChannelDelegate;
use super::color_text_label::ColorTextLabel;
use gpui::{
    div, px, AppContext, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Window,
};
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Colorize as _, Size};

pub struct HsvPhotoshopPicker {
    hsv: Hsv,
    plane: Entity<ColorFieldState>,
    slider_h: Entity<ColorSliderState>,
    slider_s: Entity<ColorSliderState>,
    slider_v: Entity<ColorSliderState>,
    _subscriptions: Vec<Subscription>,
}

impl HsvPhotoshopPicker {
    pub fn new<T>(cx: &mut Context<T>, name_prefix: &str, initial_hsv: Hsv) -> Entity<Self> {
        let plane = cx.new(|_cx| {
            ColorFieldState::hue_saturation_value(
                format!("{}_hsv_plane", name_prefix),
                initial_hsv,
                slider_constants::THUMB_SIZE_MEDIUM,
            )
            .raster_image()
            .rounded(px(0.0))
        });

        let slider_h = cx.new(|cx| {
            let mut slider =
                ColorSliderState::hue(format!("{}_hsv_slider_h", name_prefix), initial_hsv.h, cx)
                    .horizontal()
                    .rounded(px(0.0))
                    .thumb_small()
                    .thumb_square();
            slider.set_size(Size::Small, cx);
            slider
        });

        let slider_s = cx.new(|cx| {
            let mut slider = ColorSliderState::channel(
                format!("{}_hsv_slider_s", name_prefix),
                initial_hsv.s,
                ChannelDelegate::new(initial_hsv, Hsv::SATURATION.into())
                    .expect("Failed to create HSV Saturation ChannelDelegate"),
                cx,
            )
            .horizontal()
            .rounded(px(0.0))
            .thumb_small()
            .thumb_square();
            slider.set_size(Size::Small, cx);
            slider
        });

        let slider_v = cx.new(|cx| {
            let mut slider = ColorSliderState::channel(
                format!("{}_hsv_slider_v", name_prefix),
                initial_hsv.v,
                ChannelDelegate::new(initial_hsv, Hsv::VALUE.into())
                    .expect("Failed to create HSV Value ChannelDelegate"),
                cx,
            )
            .horizontal()
            .rounded(px(0.0))
            .thumb_small()
            .thumb_square();
            slider.set_size(Size::Small, cx);
            slider
        });

        cx.new(|cx| {
            let mut _subscriptions = vec![];

            let plane_clone = plane.clone();
            let slider_h_clone = slider_h.clone();
            let slider_s_clone = slider_s.clone();
            let slider_v_clone = slider_v.clone();

            _subscriptions.push(cx.subscribe(
                &plane_clone,
                move |this: &mut Self, _, event, cx| {
                    let (hue, saturation, value) = match event {
                        ColorFieldEvent::Change(hsv) | ColorFieldEvent::Release(hsv) => {
                            (hsv.h, hsv.s, hsv.v)
                        }
                    };

                    this.hsv.h = hue;
                    this.hsv.s = saturation;
                    this.hsv.v = value;

                    let hsv = this.hsv;
                    this.slider_h.update(cx, |s, cx| s.set_value(hsv.h, cx));
                    this.slider_s.update(cx, |s, cx| {
                        s.set_value(hsv.s, cx);
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(hsv, Hsv::SATURATION.into())
                                    .expect("Failed to create HSV Saturation ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    this.slider_v.update(cx, |s, cx| {
                        s.set_value(hsv.v, cx);
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(hsv, Hsv::VALUE.into())
                                    .expect("Failed to create HSV Value ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    cx.notify();
                },
            ));

            _subscriptions.push(cx.subscribe(
                &slider_h_clone,
                move |this: &mut Self, _, event, cx| {
                    let value = match event {
                        ColorSliderEvent::Change(value) => *value,
                        ColorSliderEvent::Release(value) => *value,
                    };
                    this.hsv.h = value;

                    this.plane.update(cx, |p, cx| {
                        p.set_hsv_components(this.hsv.h, this.hsv.s, this.hsv.v, cx)
                    });
                    this.slider_s.update(cx, |s, cx| {
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(this.hsv, Hsv::SATURATION.into())
                                    .expect("Failed to create HSV Saturation ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    this.slider_v.update(cx, |s, cx| {
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(this.hsv, Hsv::VALUE.into())
                                    .expect("Failed to create HSV Value ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    cx.notify();
                },
            ));

            _subscriptions.push(cx.subscribe(
                &slider_s_clone,
                move |this: &mut Self, _, event, cx| {
                    let value = match event {
                        ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
                    };
                    this.hsv.s = value;

                    this.plane.update(cx, |p, cx| {
                        p.set_hsv_components(this.hsv.h, this.hsv.s, this.hsv.v, cx)
                    });
                    this.slider_s.update(cx, |s, cx| {
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(this.hsv, Hsv::SATURATION.into())
                                    .expect("Failed to create HSV Saturation ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    this.slider_v.update(cx, |s, cx| {
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(this.hsv, Hsv::VALUE.into())
                                    .expect("Failed to create HSV Value ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    cx.notify();
                },
            ));

            _subscriptions.push(cx.subscribe(
                &slider_v_clone,
                move |this: &mut Self, _, event, cx| {
                    let value = match event {
                        ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
                    };
                    this.hsv.v = value;

                    this.plane.update(cx, |p, cx| {
                        p.set_hsv_components(this.hsv.h, this.hsv.s, this.hsv.v, cx)
                    });
                    this.slider_s.update(cx, |s, cx| {
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(this.hsv, Hsv::SATURATION.into())
                                    .expect("Failed to create HSV Saturation ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    this.slider_v.update(cx, |s, cx| {
                        s.set_delegate(
                            Box::new(
                                ChannelDelegate::new(this.hsv, Hsv::VALUE.into())
                                    .expect("Failed to create HSV Value ChannelDelegate"),
                            ),
                            cx,
                        );
                    });
                    cx.notify();
                },
            ));

            Self {
                hsv: initial_hsv,
                plane,
                slider_h,
                slider_s,
                slider_v,
                _subscriptions,
            }
        })
    }
}

impl Render for HsvPhotoshopPicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let hsv_text = format!(
            "hsv({:.0},{:.0}%,{:.0}%)",
            self.hsv.h,
            self.hsv.s * 100.0,
            self.hsv.v * 100.0
        );
        let hsla = self.hsv.to_hsla_ext();
        let hex_text = hsla.to_hex().to_uppercase();
        let plane_size = 280.0;

        v_flex()
            .w(px(plane_size))
            .gap_2()
            .child(
                div()
                    .w(px(plane_size))
                    .h(px(plane_size))
                    .overflow_hidden()
                    .child(self.plane.clone()),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(14.0))
                                    .text_size(px(8.0))
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child("H"),
                            )
                            .child(self.slider_h.clone()),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(14.0))
                                    .text_size(px(8.0))
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child("S"),
                            )
                            .child(self.slider_s.clone()),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(14.0))
                                    .text_size(px(8.0))
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child("V"),
                            )
                            .child(self.slider_v.clone()),
                    ),
            )
            .child(
                v_flex()
                    .w(px(plane_size))
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(plane_size))
                            .h(px(40.0))
                            .rounded_md()
                            .bg(hsla)
                            .border_1()
                            .border_color(cx.theme().border),
                    )
                    .child(
                        ColorTextLabel::new("hsv-photoshop-rgb-copy", hex_text.into())
                            .width_px(plane_size),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_center()
                            .text_size(px(10.0))
                            .text_color(cx.theme().muted_foreground)
                            .child(hsv_text),
                    ),
            )
    }
}
