use super::checkerboard::Checkerboard;
use super::color_readout::render_color_readout;
use super::color_ring::{
    ColorRingEvent, ColorRingRasterState, ColorRingRenderer, ColorRingState, HueRingDelegate,
};
use super::color_slider::{ColorSlider, ColorSliderEvent, ColorSliderState};
use super::color_spec::{ColorSpecification, Hsv};
use super::delegates::{ChannelDelegate, HueDelegate};
use crate::section;
use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme, Sizable};

pub struct StoryColorRingTab {
    color_ring: Entity<ColorRingState>,
    color_ring_vector_compare: Entity<ColorRingState>,
    color_ring_raster: Entity<ColorRingRasterState>,
    color_ring_vector_compare_inner_target: Entity<ColorRingState>,
    color_ring_raster_inner_target: Entity<ColorRingRasterState>,
    color_ring_saturation_vector: Entity<ColorRingState>,
    color_ring_saturation_vector_rotated: Entity<ColorRingState>,
    color_ring_saturation_raster: Entity<ColorRingRasterState>,
    color_ring_saturation_raster_rotated: Entity<ColorRingRasterState>,
    color_ring_lightness_vector: Entity<ColorRingState>,
    color_ring_lightness_raster: Entity<ColorRingRasterState>,
    color_ring_disabled_vector: Entity<ColorRingState>,
    color_ring_disabled_raster: Entity<ColorRingRasterState>,
    ring_no_border: Entity<ColorRingState>,
    ring_inner_border: Entity<ColorRingState>,
    ring_outer_border: Entity<ColorRingState>,
    ring_both_borders: Entity<ColorRingState>,
    ring_foreground_borders: Entity<ColorRingState>,
    ring_size_xsmall: Entity<ColorRingState>,
    ring_size_small: Entity<ColorRingState>,
    ring_size_medium: Entity<ColorRingState>,
    ring_size_large: Entity<ColorRingState>,
    ring_thickness_xsmall: Entity<ColorRingState>,
    ring_thickness_small: Entity<ColorRingState>,
    ring_thickness_medium: Entity<ColorRingState>,
    ring_thickness_large: Entity<ColorRingState>,
    ring_h: Entity<ColorSliderState>,
    ring_s: Entity<ColorSliderState>,
    ring_v: Entity<ColorSliderState>,
    ring_hsv: Hsv,
    ring_color: Hsla,
    ring_event_name: &'static str,
    ring_event_value: f32,
    _subscriptions: Vec<Subscription>,
}

impl StoryColorRingTab {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hsla = hsla(0.0, 1.0, 0.5, 1.0);
        let hsv = Hsv::from_hsla(hsla);
        let color_ring = cx.new(|cx| {
            ColorRingState::hue(
                "color_ring",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .allow_inner_target(true)
        });
        let color_ring_vector_compare = cx.new(|cx| {
            ColorRingState::hue_with_renderer(
                "color_ring_vector_compare",
                hsv.h,
                hsv.s,
                hsv.to_hsla().l,
                ColorRingRenderer::Vector,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });
        let color_ring_raster = cx.new(|cx| {
            ColorRingState::hue_with_renderer(
                "color_ring_raster",
                hsv.h,
                hsv.s,
                hsv.to_hsla().l,
                ColorRingRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });
        let color_ring_vector_compare_inner_target = cx.new(|cx| {
            ColorRingState::hue_with_renderer(
                "color_ring_vector_compare_inner_target",
                hsv.h,
                hsv.s,
                hsv.to_hsla().l,
                ColorRingRenderer::Vector,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .allow_inner_target(true)
        });
        let color_ring_raster_inner_target = cx.new(|cx| {
            ColorRingState::hue_with_renderer(
                "color_ring_raster_inner_target",
                hsv.h,
                hsv.s,
                hsv.to_hsla().l,
                ColorRingRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .allow_inner_target(true)
        });
        let color_ring_saturation_vector = cx.new(|cx| {
            ColorRingState::saturation_with_renderer(
                "color_ring_saturation_vector",
                0.0,
                hsv.h,
                hsv.v,
                ColorRingRenderer::Vector,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });
        let color_ring_saturation_vector_rotated = cx.new(|cx| {
            ColorRingState::saturation_with_renderer(
                "color_ring_saturation_vector_rotated",
                0.0,
                hsv.h,
                hsv.v,
                ColorRingRenderer::Vector,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .rotation_degrees(180.0)
        });
        let color_ring_saturation_raster = cx.new(|cx| {
            ColorRingState::saturation_with_renderer(
                "color_ring_saturation_raster",
                0.0,
                hsv.h,
                1.0,
                ColorRingRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });
        let color_ring_saturation_raster_rotated = cx.new(|cx| {
            ColorRingState::saturation_with_renderer(
                "color_ring_saturation_raster_rotated",
                0.0,
                hsv.h,
                1.0,
                ColorRingRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .rotation_degrees(180.0)
        });
        let color_ring_lightness_vector = cx.new(|cx| {
            ColorRingState::lightness_with_renderer(
                "color_ring_lightness_vector",
                0.0,
                hsv.h,
                hsv.s,
                ColorRingRenderer::Vector,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });
        let color_ring_lightness_raster = cx.new(|cx| {
            ColorRingState::lightness_with_renderer(
                "color_ring_lightness_raster",
                0.0,
                hsv.h,
                hsv.s,
                ColorRingRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });
        let color_ring_disabled_vector = cx.new(|cx| {
            ColorRingState::hue_with_renderer(
                "color_ring_disabled_vector",
                hsv.h,
                hsv.s,
                hsv.to_hsla().l,
                ColorRingRenderer::Vector,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .disabled(true)
        });
        let color_ring_disabled_raster = cx.new(|cx| {
            ColorRingState::hue_with_renderer(
                "color_ring_disabled_raster",
                hsv.h,
                hsv.s,
                hsv.to_hsla().l,
                ColorRingRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .disabled(true)
        });

        let ring_no_border = cx.new(|cx| {
            ColorRingState::hue(
                "ring_no_border",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Small)
            .ring_inner_border(false)
            .ring_outer_border(false)
        });

        let ring_inner_border = cx.new(|cx| {
            ColorRingState::hue(
                "ring_inner_border",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Small)
            .ring_inner_border(true)
            .ring_outer_border(false)
        });

        let ring_outer_border = cx.new(|cx| {
            ColorRingState::hue(
                "ring_outer_border",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Small)
            .ring_inner_border(false)
            .ring_outer_border(true)
        });

        let ring_both_borders = cx.new(|cx| {
            ColorRingState::hue(
                "ring_both_borders",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Small)
            .ring_inner_border(true)
            .ring_outer_border(true)
        });
        let ring_foreground_borders = cx.new(|cx| {
            ColorRingState::hue(
                "ring_foreground_borders",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Small)
            .ring_inner_border(true)
            .ring_outer_border(true)
            .ring_border_color(cx.theme().foreground)
        });

        let ring_size_xsmall = cx.new(|cx| {
            ColorRingState::hue(
                "ring_size_xsmall",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::XSmall)
        });

        let ring_size_small = cx.new(|cx| {
            ColorRingState::hue(
                "ring_size_small",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Small)
        });

        let ring_size_medium = cx.new(|cx| {
            ColorRingState::hue(
                "ring_size_medium",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Medium)
        });

        let ring_size_large = cx.new(|cx| {
            ColorRingState::hue(
                "ring_size_large",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Large)
        });

        let ring_thickness_xsmall = cx.new(|cx| {
            ColorRingState::hue(
                "ring_thickness_xsmall",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .ring_thickness_size(gpui_component::Size::XSmall)
        });

        let ring_thickness_small = cx.new(|cx| {
            ColorRingState::hue(
                "ring_thickness_small",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .ring_thickness_size(gpui_component::Size::Small)
        });

        let ring_thickness_medium = cx.new(|cx| {
            ColorRingState::hue(
                "ring_thickness_medium",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .ring_thickness_size(gpui_component::Size::Medium)
        });

        let ring_thickness_large = cx.new(|cx| {
            ColorRingState::hue(
                "ring_thickness_large",
                hsv.h,
                HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsv.to_hsla().l,
                },
                cx,
            )
            .with_size(gpui_component::Size::Medium)
            .ring_thickness_size(gpui_component::Size::Large)
        });

        let ring_h = cx.new(|cx| {
            ColorSliderState::new("ring_h", hsv.h, Box::new(HueDelegate), cx)
                .min(0.0)
                .max(360.0)
                .reversed(true)
                .vertical()
        });

        let ring_s = cx.new(|cx| {
            ColorSliderState::new(
                "ring_s",
                hsv.s,
                Box::new(ChannelDelegate {
                    spec: hsv,
                    channel_name: "saturation".into(),
                }),
                cx,
            )
            .reversed(true)
            .vertical()
        });

        let ring_v = cx.new(|cx| {
            ColorSliderState::new(
                "ring_v",
                hsv.v,
                Box::new(ChannelDelegate {
                    spec: hsv,
                    channel_name: "value".into(),
                }),
                cx,
            )
            .reversed(true)
            .vertical()
        });

        let mut _subscriptions = Vec::new();

        _subscriptions.push(cx.subscribe(&color_ring, |this, _, event, cx| {
            let (event_name, value) = match event {
                ColorRingEvent::Change(value) => ("Change", *value),
                ColorRingEvent::Release(value) => ("Release", *value),
            };
            this.ring_event_name = event_name;
            this.ring_event_value = value;
            this.ring_hsv.h = value;
            this.sync_ring(cx);
        }));
        _subscriptions.push(cx.subscribe(&ring_h, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            this.ring_hsv.h = value;
            this.sync_ring(cx);
        }));

        _subscriptions.push(cx.subscribe(&ring_s, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            this.ring_hsv.s = value;
            this.sync_ring(cx);
        }));

        _subscriptions.push(cx.subscribe(&ring_v, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            this.ring_hsv.v = value;
            this.sync_ring(cx);
        }));

        Self {
            color_ring,
            color_ring_vector_compare,
            color_ring_raster,
            color_ring_vector_compare_inner_target,
            color_ring_raster_inner_target,
            color_ring_saturation_vector,
            color_ring_saturation_vector_rotated,
            color_ring_saturation_raster,
            color_ring_saturation_raster_rotated,
            color_ring_lightness_vector,
            color_ring_lightness_raster,
            color_ring_disabled_vector,
            color_ring_disabled_raster,
            ring_no_border,
            ring_inner_border,
            ring_outer_border,
            ring_both_borders,
            ring_foreground_borders,
            ring_size_xsmall,
            ring_size_small,
            ring_size_medium,
            ring_size_large,
            ring_thickness_xsmall,
            ring_thickness_small,
            ring_thickness_medium,
            ring_thickness_large,
            ring_h,
            ring_s,
            ring_v,
            ring_hsv: hsv,
            ring_color: hsv.to_hsla(),
            ring_event_name: "Change",
            ring_event_value: hsv.h,
            _subscriptions,
        }
    }

    fn sync_ring(&mut self, cx: &mut Context<Self>) {
        let hsv = self.ring_hsv;
        let hsla = hsv.to_hsla();
        self.ring_color = hsla;

        self.color_ring.update(cx, |circle, cx| {
            circle.set_value(hsv.h, cx);
            circle.set_delegate(
                Box::new(HueRingDelegate {
                    saturation: hsv.s,
                    lightness: hsla.l,
                }),
                cx,
            );
        });
        self.ring_h.update(cx, |s, cx| s.set_value(hsv.h, cx));

        self.ring_s.update(cx, |s, cx| {
            s.set_value(hsv.s, cx);
            s.set_delegate(
                Box::new(ChannelDelegate {
                    spec: hsv,
                    channel_name: "saturation".into(),
                }),
                cx,
            );
        });

        self.ring_v.update(cx, |s, cx| {
            s.set_value(hsv.v, cx);
            s.set_delegate(
                Box::new(ChannelDelegate {
                    spec: hsv,
                    channel_name: "value".into(),
                }),
                cx,
            );
        });
    }
}

impl Render for StoryColorRingTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .child(
                section("Color Ring").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_center()
                        .gap_6()
                        .child(render_color_swatch(self.ring_color, cx))
                        .child(
                            div()
                                .size(px(256.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(render_ring_with_event(
                                    self.color_ring.clone(),
                                    self.ring_event_name,
                                    self.ring_event_value,
                                )),
                        )
                        .child(
                            h_flex()
                                .gap_4()
                                .h(px(256.0))
                                .child(ColorSlider::new(&self.ring_h))
                                .child(ColorSlider::new(&self.ring_s))
                                .child(ColorSlider::new(&self.ring_v)),
                        )
                        .child(render_color_readout(
                            self.ring_color,
                            cx.theme().mono_font_family.clone(),
                            false,
                        )),
                ),
            )
            .child(
                section("Implementation Compare").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_8()
                        .child(render_thumb_variant(
                            "Vector (paths)",
                            self.color_ring_vector_compare.clone(),
                            self.color_ring_vector_compare.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Vector (paths, inner target)",
                            self.color_ring_vector_compare_inner_target.clone(),
                            self.color_ring_vector_compare_inner_target.read(cx).value,
                        ))
                        .child(render_raster_variant(
                            "Raster (pre-imaged)",
                            self.color_ring_raster.clone(),
                            self.color_ring_raster.read(cx).value,
                        ))
                        .child(render_raster_variant(
                            "Raster (pre-imaged, inner target)",
                            self.color_ring_raster_inner_target.clone(),
                            self.color_ring_raster_inner_target.read(cx).value,
                        )),
                ),
            )
            .child(
                section("Saturation Ring (Continuous)").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_8()
                        .child(render_thumb_variant(
                            "Vector saturation",
                            self.color_ring_saturation_vector.clone(),
                            self.color_ring_saturation_vector.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Vector saturation (180°)",
                            self.color_ring_saturation_vector_rotated.clone(),
                            self.color_ring_saturation_vector_rotated.read(cx).value,
                        ))
                        .child(render_raster_variant(
                            "Raster saturation",
                            self.color_ring_saturation_raster.clone(),
                            self.color_ring_saturation_raster.read(cx).value,
                        ))
                        .child(render_raster_variant(
                            "Raster saturation (180°)",
                            self.color_ring_saturation_raster_rotated.clone(),
                            self.color_ring_saturation_raster_rotated.read(cx).value,
                        )),
                ),
            )
            .child(
                section("Lightness Ring (Continuous)").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_8()
                        .child(render_thumb_variant(
                            "Vector lightness",
                            self.color_ring_lightness_vector.clone(),
                            self.color_ring_lightness_vector.read(cx).value,
                        ))
                        .child(render_raster_variant(
                            "Raster lightness",
                            self.color_ring_lightness_raster.clone(),
                            self.color_ring_lightness_raster.read(cx).value,
                        )),
                ),
            )
            .child(
                section("Disabled").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_8()
                        .child(render_thumb_variant(
                            "Vector (disabled)",
                            self.color_ring_disabled_vector.clone(),
                            self.color_ring_disabled_vector.read(cx).value,
                        ))
                        .child(render_raster_variant(
                            "Raster (disabled)",
                            self.color_ring_disabled_raster.clone(),
                            self.color_ring_disabled_raster.read(cx).value,
                        )),
                ),
            )
            .child(
                section("Ring Border Variants").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_6()
                        .child(render_thumb_variant(
                            "no border",
                            self.ring_no_border.clone(),
                            self.ring_no_border.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "inner border",
                            self.ring_inner_border.clone(),
                            self.ring_inner_border.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "outer border",
                            self.ring_outer_border.clone(),
                            self.ring_outer_border.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "both",
                            self.ring_both_borders.clone(),
                            self.ring_both_borders.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "foreground both",
                            self.ring_foreground_borders.clone(),
                            self.ring_foreground_borders.read(cx).value,
                        )),
                ),
            )
            .child(
                section("Ring Sizes").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_6()
                        .child(render_thumb_variant(
                            "XSmall",
                            self.ring_size_xsmall.clone(),
                            self.ring_size_xsmall.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Small",
                            self.ring_size_small.clone(),
                            self.ring_size_small.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Medium",
                            self.ring_size_medium.clone(),
                            self.ring_size_medium.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Large",
                            self.ring_size_large.clone(),
                            self.ring_size_large.read(cx).value,
                        )),
                ),
            )
            .child(
                section("Ring Thickness Sizes").max_w_full().child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .items_start()
                        .gap_6()
                        .child(render_thumb_variant(
                            "XSmall",
                            self.ring_thickness_xsmall.clone(),
                            self.ring_thickness_xsmall.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Small",
                            self.ring_thickness_small.clone(),
                            self.ring_thickness_small.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Medium",
                            self.ring_thickness_medium.clone(),
                            self.ring_thickness_medium.read(cx).value,
                        ))
                        .child(render_thumb_variant(
                            "Large",
                            self.ring_thickness_large.clone(),
                            self.ring_thickness_large.read(cx).value,
                        )),
                ),
            )
    }
}

fn render_color_swatch(color: gpui::Hsla, cx: &mut Context<StoryColorRingTab>) -> impl IntoElement {
    div().size(px(220.0)).rounded_lg().overflow_hidden().child(
        Checkerboard::new(cx.theme().is_dark()).child(
            div()
                .size_full()
                .bg(color)
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border),
        ),
    )
}

fn format_ring_value(value: f32) -> String {
    if value.abs() >= 10.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn render_ring_with_value(
    circle: Entity<ColorRingState>,
    value: f32,
    is_vector: bool,
) -> impl IntoElement {
    let label = if is_vector {
        format!("*{}", format_ring_value(value))
    } else {
        format_ring_value(value)
    };

    div().relative().child(circle).child(
        div()
            .absolute()
            .left_1_2()
            .top_1_2()
            .ml(px(-24.0))
            .mt(px(-10.0))
            .w(px(48.0))
            .h(px(20.0))
            .rounded_sm()
            .bg(black().opacity(0.45))
            .text_xs()
            .text_color(white())
            .flex()
            .items_center()
            .justify_center()
            .child(label),
    )
}

fn render_ring_with_event(
    circle: Entity<ColorRingState>,
    event_name: &'static str,
    event_value: f32,
) -> impl IntoElement {
    div().relative().child(circle).child(
        v_flex()
            .absolute()
            .left_1_2()
            .top_1_2()
            .ml(px(-42.0))
            .mt(px(-16.0))
            .w(px(84.0))
            .h(px(32.0))
            .rounded_sm()
            .bg(black().opacity(0.45))
            .text_xs()
            .text_color(white())
            .items_center()
            .justify_center()
            .child(div().child(event_name))
            .child(div().text_sm().child(format_ring_value(event_value))),
    )
}

fn render_raster_ring_with_value(
    circle: Entity<ColorRingRasterState>,
    value: f32,
) -> impl IntoElement {
    div().relative().child(circle).child(
        div()
            .absolute()
            .left_1_2()
            .top_1_2()
            .ml(px(-24.0))
            .mt(px(-10.0))
            .w(px(48.0))
            .h(px(20.0))
            .rounded_sm()
            .bg(black().opacity(0.45))
            .text_xs()
            .text_color(white())
            .flex()
            .items_center()
            .justify_center()
            .child(format_ring_value(value)),
    )
}

fn render_thumb_variant(
    label: &'static str,
    circle: Entity<ColorRingState>,
    value: f32,
) -> impl IntoElement {
    v_flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().child(label))
        .child(render_ring_with_value(circle, value, true))
}

fn render_raster_variant(
    label: &'static str,
    circle: Entity<ColorRingRasterState>,
    value: f32,
) -> impl IntoElement {
    v_flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().child(label))
        .child(render_raster_ring_with_value(circle, value))
}
