use super::color_field::{ColorFieldEvent, ColorFieldState, PolygonDomain, SvAtHueModel};
use super::color_slider::{ColorSliderEvent, ColorSliderState};
use super::color_spec::Hsv;
use crate::section;
use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Size,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use plane_panel::StoryColorPlanePanel;
use std::sync::Arc;
use wheel_panel::StoryColorWheelPanel;

const FIELD_WIDTH: f32 = 220.0;
const FIELD_HEIGHT: f32 = 220.0;
const VALUE_FONT_SIZE: f32 = 10.0;

pub struct StoryColorFieldTab {
    hue_slider: Entity<ColorSliderState>,
    rect_field: Entity<ColorFieldState>,
    triangle_field: Entity<ColorFieldState>,
    polygon_field: Entity<ColorFieldState>,
    circle_field: Entity<ColorFieldState>,
    rect_raster_field: Entity<ColorFieldState>,
    triangle_raster_field: Entity<ColorFieldState>,
    polygon_raster_field: Entity<ColorFieldState>,
    circle_raster_field: Entity<ColorFieldState>,
    rect_hsv: Hsv,
    triangle_hsv: Hsv,
    polygon_hsv: Hsv,
    circle_hsv: Hsv,
    rect_raster_hsv: Hsv,
    triangle_raster_hsv: Hsv,
    polygon_raster_hsv: Hsv,
    circle_raster_hsv: Hsv,
    wheel_panel: Entity<StoryColorWheelPanel>,
    plane_panel: Entity<StoryColorPlanePanel>,
    domain_swaps_rendered: bool,
    _subscriptions: Vec<Subscription>,
}

impl StoryColorFieldTab {
    pub fn view(window: &mut Window, cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_hsv = Hsv::from_hsla_ext(gpui::hsla(208.0 / 360.0, 0.75, 0.47, 1.0));

        let rect_field = cx.new(|_cx| {
            ColorFieldState::saturation_value_rect("color_field_rect", initial_hsv, 16.0).vector()
        });
        let triangle_field = cx.new(|_cx| {
            ColorFieldState::saturation_value_triangle("color_field_triangle", initial_hsv, 16.0)
                .vector()
        });
        let polygon_field = cx.new(|_cx| {
            ColorFieldState::new(
                "color_field_polygon",
                initial_hsv,
                Arc::new(PolygonDomain::new(vec![
                    (0.12, 0.08),
                    (0.88, 0.17),
                    (0.94, 0.66),
                    (0.60, 0.96),
                    (0.16, 0.86),
                    (0.06, 0.42),
                ])),
                Arc::new(SvAtHueModel),
            )
            .thumb_size(16.0)
            .vector()
        });
        let circle_field = cx.new(|_cx| {
            ColorFieldState::hue_saturation_wheel("color_field_circle", initial_hsv, 16.0).vector()
        });
        let rect_raster_field = cx.new(|_cx| {
            ColorFieldState::saturation_value_rect("color_field_rect_raster", initial_hsv, 16.0)
                .raster_image_prewarmed_square(FIELD_WIDTH)
        });
        let triangle_raster_field = cx.new(|_cx| {
            ColorFieldState::saturation_value_triangle(
                "color_field_triangle_raster",
                initial_hsv,
                16.0,
            )
            .raster_image_prewarmed_square(FIELD_WIDTH)
        });
        let polygon_raster_field = cx.new(|_cx| {
            ColorFieldState::new(
                "color_field_polygon_raster",
                initial_hsv,
                Arc::new(PolygonDomain::new(vec![
                    (0.12, 0.08),
                    (0.88, 0.17),
                    (0.94, 0.66),
                    (0.60, 0.96),
                    (0.16, 0.86),
                    (0.06, 0.42),
                ])),
                Arc::new(SvAtHueModel),
            )
            .thumb_size(16.0)
            .raster_image_prewarmed_square(FIELD_WIDTH)
        });
        let circle_raster_field = cx.new(|_cx| {
            ColorFieldState::hue_saturation_wheel("color_field_circle_raster", initial_hsv, 16.0)
                .raster_image_prewarmed_square(FIELD_WIDTH)
        });

        let hue_slider = cx.new(|cx| {
            let mut slider = ColorSliderState::hue("color_field_hue_axis", initial_hsv.h, cx)
                .horizontal()
                .thumb_square()
                .thumb_small();
            slider.set_size(Size::Small, cx);
            slider
        });
        let wheel_panel = StoryColorWheelPanel::view(_window, cx);
        let plane_panel = cx.new(|cx| StoryColorPlanePanel::new(_window, cx));

        let mut subscriptions = Vec::new();

        subscriptions.push(cx.subscribe(&hue_slider, |this, _, event, cx| {
            let (hue, should_notify_parent) = match event {
                ColorSliderEvent::Change(value) => (*value, false),
                ColorSliderEvent::Release(value) => (*value, true),
            };

            this.rect_hsv.h = hue;
            this.triangle_hsv.h = hue;
            this.polygon_hsv.h = hue;
            this.circle_hsv.h = hue;
            this.rect_raster_hsv.h = hue;
            this.triangle_raster_hsv.h = hue;
            this.polygon_raster_hsv.h = hue;
            this.circle_raster_hsv.h = hue;

            let rect_hsv = this.rect_hsv;
            let triangle_hsv = this.triangle_hsv;
            let polygon_hsv = this.polygon_hsv;
            let circle_hsv = this.circle_hsv;
            let rect_raster_hsv = this.rect_raster_hsv;
            let triangle_raster_hsv = this.triangle_raster_hsv;
            let polygon_raster_hsv = this.polygon_raster_hsv;
            let circle_raster_hsv = this.circle_raster_hsv;

            this.rect_field
                .update(cx, |field, cx| field.set_hsv(rect_hsv, cx));
            this.triangle_field
                .update(cx, |field, cx| field.set_hsv(triangle_hsv, cx));
            this.polygon_field
                .update(cx, |field, cx| field.set_hsv(polygon_hsv, cx));
            this.circle_field
                .update(cx, |field, cx| field.set_hsv(circle_hsv, cx));
            this.rect_raster_field
                .update(cx, |field, cx| field.set_hsv(rect_raster_hsv, cx));
            this.triangle_raster_field
                .update(cx, |field, cx| field.set_hsv(triangle_raster_hsv, cx));
            this.polygon_raster_field
                .update(cx, |field, cx| field.set_hsv(polygon_raster_hsv, cx));
            this.circle_raster_field
                .update(cx, |field, cx| field.set_hsv(circle_raster_hsv, cx));

            if should_notify_parent {
                cx.notify();
            }
        }));

        subscriptions.push(cx.subscribe(&rect_field, |this, _, event, cx| match event {
            ColorFieldEvent::Change(hsv) => {
                this.rect_hsv = *hsv;
            }
            ColorFieldEvent::Release(hsv) => {
                this.rect_hsv = *hsv;
                cx.notify();
            }
        }));
        subscriptions.push(
            cx.subscribe(&triangle_field, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.triangle_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.triangle_hsv = *hsv;
                    cx.notify();
                }
            }),
        );
        subscriptions.push(
            cx.subscribe(&polygon_field, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.polygon_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.polygon_hsv = *hsv;
                    cx.notify();
                }
            }),
        );
        subscriptions.push(
            cx.subscribe(&circle_field, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.circle_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.circle_hsv = *hsv;
                    cx.notify();
                }
            }),
        );
        subscriptions.push(
            cx.subscribe(&rect_raster_field, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.rect_raster_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.rect_raster_hsv = *hsv;
                    cx.notify();
                }
            }),
        );
        subscriptions.push(cx.subscribe(
            &triangle_raster_field,
            |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.triangle_raster_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.triangle_raster_hsv = *hsv;
                    cx.notify();
                }
            },
        ));
        subscriptions.push(
            cx.subscribe(&polygon_raster_field, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.polygon_raster_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.polygon_raster_hsv = *hsv;
                    cx.notify();
                }
            }),
        );
        subscriptions.push(
            cx.subscribe(&circle_raster_field, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) => {
                    this.circle_raster_hsv = *hsv;
                }
                ColorFieldEvent::Release(hsv) => {
                    this.circle_raster_hsv = *hsv;
                    cx.notify();
                }
            }),
        );

        Self {
            hue_slider,
            rect_field,
            triangle_field,
            polygon_field,
            circle_field,
            rect_raster_field,
            triangle_raster_field,
            polygon_raster_field,
            circle_raster_field,
            rect_hsv: initial_hsv,
            triangle_hsv: initial_hsv,
            polygon_hsv: initial_hsv,
            circle_hsv: initial_hsv,
            rect_raster_hsv: initial_hsv,
            triangle_raster_hsv: initial_hsv,
            polygon_raster_hsv: initial_hsv,
            circle_raster_hsv: initial_hsv,
            wheel_panel,
            plane_panel,
            domain_swaps_rendered: false,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for StoryColorFieldTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let intro_section = section("Computational 2D Color Field").max_w_full().child(
            v_flex()
                .w_full()
                .gap_3()
                .child(
                    div()
                        .text_size(px(VALUE_FONT_SIZE))
                        .text_color(cx.theme().muted_foreground)
                        .child("Rect/Triangle/Polygon: SV-at-hue. Circle: hue+saturation wheel at fixed value."),
                )
                .child(self.hue_slider.clone()),
        );

        let domains_section = section("Domain Swaps (Vector)").max_w_full().child(
            h_flex()
                .w_full()
                .p_4()
                .justify_center()
                .items_start()
                .flex_wrap()
                .gap(px(32.0))
                .child(render_field_card(
                    "RectDomain",
                    self.rect_field.clone(),
                    self.rect_hsv,
                    cx,
                ))
                .child(render_field_card(
                    "TriangleDomain",
                    self.triangle_field.clone(),
                    self.triangle_hsv,
                    cx,
                ))
                .child(render_field_card(
                    "PolygonDomain",
                    self.polygon_field.clone(),
                    self.polygon_hsv,
                    cx,
                ))
                .child(render_field_card(
                    "CircleDomain (Wheel)",
                    self.circle_field.clone(),
                    self.circle_hsv,
                    cx,
                )),
        );
        let raster_domains_section = section("Domain Swaps (Rasterized/Cached)")
            .max_w_full()
            .child(
                h_flex()
                    .w_full()
                    .p_4()
                    .justify_center()
                    .items_start()
                    .flex_wrap()
                    .gap(px(32.0))
                    .child(render_field_card(
                        "RectDomain",
                        self.rect_raster_field.clone(),
                        self.rect_raster_hsv,
                        cx,
                    ))
                    .child(render_field_card(
                        "TriangleDomain",
                        self.triangle_raster_field.clone(),
                        self.triangle_raster_hsv,
                        cx,
                    ))
                    .child(render_field_card(
                        "PolygonDomain",
                        self.polygon_raster_field.clone(),
                        self.polygon_raster_hsv,
                        cx,
                    ))
                    .child(render_field_card(
                        "CircleDomain (Wheel)",
                        self.circle_raster_field.clone(),
                        self.circle_raster_hsv,
                        cx,
                    )),
            );

        let domains_gate_section = section("Domain Swaps (Drawn)").max_w_full().child(
            div()
                .w_full()
                .p_6()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .w(px(320.0))
                        .max_w_full()
                        .gap_3()
                        .p_4()
                        .rounded(px(8.0))
                        .bg(cx.theme().background)
                        .border_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .text_size(px(VALUE_FONT_SIZE))
                                .text_color(cx.theme().muted_foreground)
                                .child("Check application standard output, on Mac (M4 Max 128GB mem.), you might see this error:"),
                        )
                        .child(
                            div()
                                .text_size(px(VALUE_FONT_SIZE))
                                .text_color(cx.theme().muted_foreground)
                                .child("gpui::platform::mac::metal_renderer: failed to render: scene too large: 12 paths, 0 shadows, 26021 quads, 0 underlines, 832 mono, 4 poly, 0 surfaces. retrying with larger instance buffer size"),
                        )
                        .child(
                            Button::new("color-field-domain-swaps-render")
                                .warning()
                                .label("Render")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.domain_swaps_rendered = true;
                                    cx.notify();
                                })),
                        ),
                ),
        );

        v_flex()
            .w_full()
            .gap_6()
            .child(intro_section)
            .child(if self.domain_swaps_rendered {
                domains_section.into_any_element()
            } else {
                domains_gate_section.into_any_element()
            })
            .child(raster_domains_section)
            .child(self.wheel_panel.clone())
            .child(self.plane_panel.clone())
    }
}

fn render_field_card(
    title: impl Into<SharedString>,
    field: Entity<ColorFieldState>,
    hsv: Hsv,
    cx: &mut Context<StoryColorFieldTab>,
) -> impl IntoElement {
    let title = title.into();
    let hsla = hsv.to_hsla_ext();
    let label = format!(
        "h:{:.0} s:{:.0}% v:{:.0}%",
        hsv.h,
        hsv.s * 100.0,
        hsv.v * 100.0
    );

    v_flex()
        .w(px(FIELD_WIDTH))
        .gap_2()
        .child(
            div()
                .text_size(px(VALUE_FONT_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(div().w(px(FIELD_WIDTH)).h(px(FIELD_HEIGHT)).child(field))
        .child(
            div()
                .text_size(px(VALUE_FONT_SIZE))
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(
            div()
                .w(px(FIELD_WIDTH))
                .h(px(28.0))
                .rounded(px(6.0))
                .bg(hsla)
                .border_1()
                .border_color(cx.theme().border),
        )
}

mod wheel_panel {
use super::super::checkerboard::Checkerboard;
use super::super::color_field::{
    CircleDomain, ColorFieldEvent, ColorFieldModel2D, ColorFieldState, FieldThumbPosition,
    GammaCorrectedHsvWheelModel, HslWheelModel, OklchWheelModel, WhiteMixHueWheelModel,
};
use super::super::color_readout::render_color_readout;
use super::super::color_slider::{ColorSlider, ColorSliderEvent, ColorSliderState};
use super::super::color_spec::{ColorSpecification, Hsl};
use super::super::delegates::{ChannelDelegate, HueDelegate};
use super::super::compositions::hsl_wheel_carrier::HslWheelCarrier;
use crate::section;
use gpui::*;
use gpui_component::{button::Button, h_flex, switch::Switch, v_flex, ActiveTheme};
use std::sync::Arc;

const WHEEL_XSMALL_SIZE_PX: f32 = 140.0;
const WHEEL_SMALL_SIZE_PX: f32 = 180.0;
const WHEEL_MEDIUM_SIZE_PX: f32 = 220.0;
const WHEEL_LARGE_SIZE_PX: f32 = 280.0;
const WHEEL_MAIN_SIZE_PX: f32 = 256.0;

pub struct StoryColorWheelPanel {
    color_wheel: Entity<ColorFieldState>,
    wheel_xsmall: Entity<ColorFieldState>,
    wheel_small: Entity<ColorFieldState>,
    wheel_medium: Entity<ColorFieldState>,
    wheel_large: Entity<ColorFieldState>,
    wheel_compare_white_mix: Entity<ColorFieldState>,
    wheel_compare_hsl: Entity<ColorFieldState>,
    wheel_compare_gamma_hsv: Entity<ColorFieldState>,
    wheel_compare_oklch: Entity<ColorFieldState>,
    wheel_h: Entity<ColorSliderState>,
    wheel_s: Entity<ColorSliderState>,
    wheel_l: Entity<ColorSliderState>,
    wheel_color: Hsla,
    wheel_show_border: bool,
    wheel_inside_mode: bool,
    wheel_event_name: &'static str,
    wheel_event_value: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl StoryColorWheelPanel {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hsla = hsla(0.0, 1.0, 0.5, 1.0);
        let hsl = Hsl::from_hsla(hsla);

        let color_wheel = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel",
                hsla,
                16.0,
                Arc::new(HslWheelModel),
                true,
                WHEEL_MAIN_SIZE_PX,
            )
        });
        let wheel_xsmall = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_xsmall",
                hsla,
                12.0,
                Arc::new(WhiteMixHueWheelModel),
                true,
                WHEEL_XSMALL_SIZE_PX,
            )
        });
        let wheel_small = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_small",
                hsla,
                14.0,
                Arc::new(WhiteMixHueWheelModel),
                true,
                WHEEL_SMALL_SIZE_PX,
            )
        });
        let wheel_medium = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_medium",
                hsla,
                16.0,
                Arc::new(WhiteMixHueWheelModel),
                true,
                WHEEL_MEDIUM_SIZE_PX,
            )
        });
        let wheel_large = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_large",
                hsla,
                18.0,
                Arc::new(WhiteMixHueWheelModel),
                true,
                WHEEL_LARGE_SIZE_PX,
            )
        });
        let wheel_compare_white_mix = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_compare_white_mix",
                hsla,
                12.0,
                Arc::new(WhiteMixHueWheelModel),
                true,
                WHEEL_XSMALL_SIZE_PX,
            )
        });
        let wheel_compare_hsl = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_compare_hsl",
                hsla,
                12.0,
                Arc::new(HslWheelModel),
                true,
                WHEEL_XSMALL_SIZE_PX,
            )
        });
        let wheel_compare_gamma_hsv = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_compare_gamma_hsv",
                hsla,
                12.0,
                Arc::new(GammaCorrectedHsvWheelModel),
                true,
                WHEEL_XSMALL_SIZE_PX,
            )
        });
        let wheel_compare_oklch = cx.new(|_cx| {
            new_field_wheel(
                "color_wheel_compare_oklch",
                hsla,
                12.0,
                Arc::new(OklchWheelModel),
                true,
                WHEEL_XSMALL_SIZE_PX,
            )
        });

        let wheel_h = cx.new(|cx| {
            ColorSliderState::new("wheel_h", hsla.h * 360.0, Box::new(HueDelegate), cx)
                .min(0.0)
                .max(360.0)
                .reversed(true)
                .vertical()
        });

        let wheel_s = cx.new(|cx| {
            ColorSliderState::new(
                "wheel_s",
                hsla.s,
                Box::new(ChannelDelegate {
                    spec: hsl,
                    channel_name: "saturation".into(),
                }),
                cx,
            )
            .reversed(true)
            .vertical()
        });

        let wheel_l = cx.new(|cx| {
            ColorSliderState::new(
                "wheel_l",
                hsla.l,
                Box::new(ChannelDelegate {
                    spec: hsl,
                    channel_name: "lightness".into(),
                }),
                cx,
            )
            .reversed(true)
            .vertical()
        });

        let mut _subscriptions = Vec::new();

        _subscriptions.push(
            cx.subscribe(&color_wheel, |this, _, event, cx| match event {
                ColorFieldEvent::Change(hsv) | ColorFieldEvent::Release(hsv) => {
                    let wheel_hsl = HslWheelCarrier::from_field_hsv(*hsv);
                    this.wheel_event_name = match event {
                        ColorFieldEvent::Change(_) => "Change",
                        ColorFieldEvent::Release(_) => "Release",
                    };
                    this.wheel_event_value = format_wheel_event_value(wheel_hsl);
                    this.wheel_color = wheel_hsl.to_hsla();
                    this.sync_wheel(cx);
                    cx.notify();
                }
            }),
        );

        _subscriptions.push(cx.subscribe(&wheel_h, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            let mut hsla = this.wheel_color;
            hsla.h = value / 360.0;
            this.wheel_color = hsla;
            this.sync_wheel(cx);
        }));

        _subscriptions.push(cx.subscribe(&wheel_s, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            let mut hsla = this.wheel_color;
            hsla.s = value;
            this.wheel_color = hsla;
            this.sync_wheel(cx);
        }));

        _subscriptions.push(cx.subscribe(&wheel_l, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            let mut hsla = this.wheel_color;
            hsla.l = value;
            this.wheel_color = hsla;
            this.sync_wheel(cx);
        }));

        Self {
            color_wheel,
            wheel_xsmall,
            wheel_small,
            wheel_medium,
            wheel_large,
            wheel_compare_white_mix,
            wheel_compare_hsl,
            wheel_compare_gamma_hsv,
            wheel_compare_oklch,
            wheel_h,
            wheel_s,
            wheel_l,
            wheel_color: hsla,
            wheel_show_border: true,
            wheel_inside_mode: true,
            wheel_event_name: "Change",
            wheel_event_value: format!("h:{:.1} s:{:.3} l:{:.3}", hsla.h * 360.0, hsla.s, hsla.l)
                .into(),
            _subscriptions,
        }
    }

    fn sync_wheel(&mut self, cx: &mut Context<Self>) {
        let hsla = self.wheel_color;
        let hsl = Hsl::from_hsla(hsla);
        let wheel_field_hsv_carrier = HslWheelCarrier::from_hsla(hsla).into_field_hsv();

        self.color_wheel
            .update(cx, |w, cx| w.set_hsv(wheel_field_hsv_carrier, cx));

        self.wheel_h
            .update(cx, |s, cx| s.set_value(hsla.h * 360.0, cx));

        self.wheel_s.update(cx, |s, cx| {
            s.set_value(hsla.s, cx);
            s.set_delegate(
                Box::new(ChannelDelegate {
                    spec: hsl,
                    channel_name: "saturation".into(),
                }),
                cx,
            );
        });

        self.wheel_l.update(cx, |s, cx| {
            s.set_value(hsla.l, cx);
            s.set_delegate(
                Box::new(ChannelDelegate {
                    spec: hsl,
                    channel_name: "lightness".into(),
                }),
                cx,
            );
        });
    }

    fn apply_wheel_controls(&mut self, cx: &mut Context<Self>) {
        let thumb_position = if self.wheel_inside_mode {
            FieldThumbPosition::InsideField
        } else {
            FieldThumbPosition::EdgeToEdge
        };
        self.color_wheel.update(cx, |wheel, cx| {
            wheel.set_show_border(self.wheel_show_border, cx);
            wheel.set_thumb_position(thumb_position, cx);
        });
    }

    fn reset_wheel_defaults(&mut self, cx: &mut Context<Self>) {
        let hsla = hsla(0.0, 1.0, 0.5, 1.0);
        self.wheel_color = hsla;
        self.wheel_show_border = true;
        self.wheel_inside_mode = true;
        self.wheel_event_name = "Change";
        self.wheel_event_value =
            format!("h:{:.1} s:{:.3} l:{:.3}", hsla.h * 360.0, hsla.s, hsla.l).into();
        self.sync_wheel(cx);
        self.apply_wheel_controls(cx);
        cx.notify();
    }
}

impl Render for StoryColorWheelPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let wheel_section = section("Color Wheel: Hue, Saturation & Lightness (HSL)")
            .max_w_full()
            .child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .items_center()
                    .gap_6()
                    .child(
                        v_flex()
                            .w(px(180.0))
                            .gap_2()
                            .p_3()
                            .rounded(px(6.0))
                            .bg(cx.theme().background.opacity(0.72))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(div().text_size(px(9.0)).child("Controls"))
                            .child(
                                Switch::new("wheel-main-border")
                                    .checked(self.wheel_show_border)
                                    .label("Border")
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.wheel_show_border = *checked;
                                        this.apply_wheel_controls(cx);
                                    })),
                            )
                            .child(
                                Switch::new("wheel-main-inside")
                                    .checked(self.wheel_inside_mode)
                                    .label("Inside")
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.wheel_inside_mode = *checked;
                                        this.apply_wheel_controls(cx);
                                    })),
                            )
                            .child(
                                Button::new("wheel-main-reset")
                                    .outline()
                                    .label("Reset")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.reset_wheel_defaults(cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .size(px(WHEEL_MAIN_SIZE_PX))
                            .child(self.color_wheel.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .h(px(WHEEL_MAIN_SIZE_PX))
                            .child(ColorSlider::new(&self.wheel_h))
                            .child(ColorSlider::new(&self.wheel_s))
                            .child(ColorSlider::new(&self.wheel_l)),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(render_color_swatch(self.wheel_color, cx))
                            .child(render_color_readout(
                                self.wheel_color,
                                cx.theme().mono_font_family.clone(),
                                false,
                            ))
                            .child(render_wheel_event_readout(
                                self.wheel_event_name,
                                self.wheel_event_value.clone(),
                                cx.theme().mono_font_family.clone(),
                                cx.theme().border,
                                cx.theme().background.opacity(0.72),
                            )),
                    ),
            );

        let wheel_sizes_section = section("Wheel Sizes").max_w_full().child(
            h_flex()
                .w_full()
                .justify_center()
                .items_start()
                .gap_6()
                .child(render_wheel_size_variant(
                    "XSmall",
                    self.wheel_xsmall.clone(),
                    WHEEL_XSMALL_SIZE_PX,
                ))
                .child(render_wheel_size_variant(
                    "Small",
                    self.wheel_small.clone(),
                    WHEEL_SMALL_SIZE_PX,
                ))
                .child(render_wheel_size_variant(
                    "Medium",
                    self.wheel_medium.clone(),
                    WHEEL_MEDIUM_SIZE_PX,
                ))
                .child(render_wheel_size_variant(
                    "Large",
                    self.wheel_large.clone(),
                    WHEEL_LARGE_SIZE_PX,
                )),
        );

        let rendering_specs_section = section("Rendering Specs (Side by Side)")
            .max_w_full()
            .child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .items_start()
                    .gap_6()
                    .child(render_wheel_size_variant(
                        "White Mix (common)",
                        self.wheel_compare_white_mix.clone(),
                        WHEEL_XSMALL_SIZE_PX,
                    ))
                    .child(render_wheel_size_variant(
                        "HSL (uniform radial S)",
                        self.wheel_compare_hsl.clone(),
                        WHEEL_XSMALL_SIZE_PX,
                    ))
                    .child(render_wheel_size_variant(
                        "Gamma-corrected HSV",
                        self.wheel_compare_gamma_hsv.clone(),
                        WHEEL_XSMALL_SIZE_PX,
                    ))
                    .child(render_wheel_size_variant(
                        "OKLCH (visual smooth)",
                        self.wheel_compare_oklch.clone(),
                        WHEEL_XSMALL_SIZE_PX,
                    )),
            );

        v_flex()
            .size_full()
            .gap_6()
            .child(wheel_section)
            .child(wheel_sizes_section)
            .child(rendering_specs_section)
    }
}

fn render_color_swatch(
    color: gpui::Hsla,
    cx: &mut Context<StoryColorWheelPanel>,
) -> impl IntoElement {
    div()
        .w(px(250.0))
        .h(px(64.0))
        .rounded(px(4.0))
        .overflow_hidden()
        .child(
            Checkerboard::new(cx.theme().is_dark()).child(
                div()
                    .size_full()
                    .bg(color)
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(cx.theme().border),
            ),
        )
}

fn render_wheel_size_variant(
    label: &'static str,
    wheel: Entity<ColorFieldState>,
    size_px: f32,
) -> impl IntoElement {
    v_flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().child(label))
        .child(div().size(px(size_px)).child(wheel))
}

fn format_wheel_event_value(wheel_hsl: HslWheelCarrier) -> SharedString {
    format!(
        "h:{:.1} s:{:.3} l:{:.3}",
        wheel_hsl.hue_degrees.clamp(0.0, 360.0),
        wheel_hsl.saturation.clamp(0.0, 1.0),
        wheel_hsl.lightness.clamp(0.0, 1.0),
    )
    .into()
}

fn new_field_wheel(
    id: impl Into<SharedString>,
    hsla: Hsla,
    thumb_size: f32,
    model: Arc<dyn ColorFieldModel2D>,
    inside_thumb: bool,
    prewarm_size_px: f32,
) -> ColorFieldState {
    let mut wheel = ColorFieldState::new(
        id,
        HslWheelCarrier::from_hsla(hsla).into_field_hsv(),
        Arc::new(CircleDomain),
        model,
    )
    .thumb_size(thumb_size)
    .raster_image_prewarmed_square(prewarm_size_px);

    wheel = if inside_thumb {
        wheel.inside_field()
    } else {
        wheel.edge_to_edge()
    };

    wheel
}

fn render_wheel_event_readout(
    event_name: &'static str,
    event_value: SharedString,
    mono_font_family: SharedString,
    border_color: Hsla,
    background_color: Hsla,
) -> impl IntoElement {
    v_flex()
        .w(px(250.0))
        .gap_1()
        .p_2()
        .rounded(px(4.0))
        .bg(background_color)
        .border_1()
        .border_color(border_color)
        .font_family(mono_font_family)
        .child(
            div()
                .text_size(px(11.0))
                .child(format!("Wheel {event_name}")),
        )
        .child(div().text_size(px(11.0)).child(event_value))
}
}

mod plane_panel {
use super::super::checkerboard::Checkerboard;
use super::super::color_field::{ColorFieldEvent, ColorFieldState, FieldThumbPosition};
use super::super::color_slider::sizing as slider_constants;
use super::super::color_slider::{ColorSliderEvent, ColorSliderState};
use super::super::color_spec::Hsv;
use super::super::delegates::ChannelDelegate;
use crate::section;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AppContext, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Window,
};
use gpui_component::{h_flex, switch::Switch, v_flex, ActiveTheme as _};

const COLOR_VALUE_FONT_SIZE: f32 = 9.0;
const PLANE_DIMENSION_XSMALL: f32 = 140.0;
const PLANE_DIMENSION_SMALL: f32 = 180.0;
const PLANE_DIMENSION_MEDIUM: f32 = 220.0;
const PLANE_DIMENSION_LARGE: f32 = 280.0;

#[derive(Clone, Copy)]
enum PlaneMode {
    SvAtHue,        // Standard HSV: Plane=SV, Slider=H
    HsAtValue,      // Plane=HS, Slider=V
    HvAtSaturation, // Plane=HV, Slider=S
}

struct ColorPlaneTestPicker {
    label: Option<String>,
    hsv: Hsv,
    plane: Entity<ColorFieldState>,
    slider: Entity<ColorSliderState>,
    mode: PlaneMode,
    plane_width: f32,
    plane_height: f32,
    clip_corner_radius: f32,
    show_checkerboard: bool,
    thumb_position: FieldThumbPosition,
    last_event_source: &'static str,
    last_event_name: &'static str,
    last_event_value: String,
    _subscriptions: Vec<Subscription>,
}

impl ColorPlaneTestPicker {
    fn new(
        cx: &mut Context<StoryColorPlanePanel>,
        label: Option<impl Into<String>>,
        mode: PlaneMode,
        name_prefix: &str,
        initial_hsv: Hsv,
        plane_corner_radius: f32,
        slider_corner_radius: f32,
        with_border: bool,
        plane_width: f32,
        plane_height: f32,
        show_checkerboard: bool,
        thumb_position: FieldThumbPosition,
    ) -> Entity<Self> {
        let plane = cx.new(|_cx| {
            let mut state = match mode {
                PlaneMode::SvAtHue => ColorFieldState::saturation_value(
                    format!("{}_plane", name_prefix),
                    initial_hsv,
                    slider_constants::THUMB_SIZE_MEDIUM,
                ),
                PlaneMode::HsAtValue => ColorFieldState::hue_saturation(
                    format!("{}_plane", name_prefix),
                    initial_hsv,
                    slider_constants::THUMB_SIZE_MEDIUM,
                ),
                PlaneMode::HvAtSaturation => ColorFieldState::hue_value(
                    format!("{}_plane", name_prefix),
                    initial_hsv,
                    slider_constants::THUMB_SIZE_MEDIUM,
                ),
            };
            state = state.rounded(px(plane_corner_radius));
            if !with_border {
                state = state.no_border();
            }
            state.thumb_position = thumb_position;
            state = state.raster_image_prewarmed(plane_width, plane_height);
            state
        });

        let slider = cx.new(|cx| {
            let slider = match mode {
                PlaneMode::SvAtHue => ColorSliderState::hue(
                    format!("{}_plane_slider", name_prefix),
                    initial_hsv.h,
                    cx,
                ),
                PlaneMode::HsAtValue => ColorSliderState::channel(
                    format!("{}_plane_slider", name_prefix),
                    initial_hsv.v,
                    ChannelDelegate::new(initial_hsv, Hsv::VALUE.into())
                        .expect("Failed to create Value ChannelDelegate"),
                    cx,
                ),
                PlaneMode::HvAtSaturation => ColorSliderState::channel(
                    format!("{}_plane_slider", name_prefix),
                    initial_hsv.s,
                    ChannelDelegate::new(initial_hsv, Hsv::SATURATION.into())
                        .expect("Failed to create Saturation ChannelDelegate"),
                    cx,
                ),
            };
            slider
                .horizontal()
                .rounded(px(slider_corner_radius))
                .thumb_square()
        });

        let label = label.map(|l| l.into());

        cx.new(|cx| {
            let mut _subscriptions = vec![];

            let plane_clone = plane.clone();
            let slider_clone = slider.clone();

            _subscriptions.push(cx.subscribe(
                &plane_clone,
                move |this: &mut Self, _, event, cx| {
                    let (event_name, hue, saturation, value) = match event {
                        ColorFieldEvent::Change(hsv) => ("Change", hsv.h, hsv.s, hsv.v),
                        ColorFieldEvent::Release(hsv) => ("Release", hsv.h, hsv.s, hsv.v),
                    };
                    this.hsv.h = hue;
                    this.hsv.s = saturation;
                    this.hsv.v = value;
                    let hsla = this.hsv.to_hsla_ext();
                    this.last_event_source = "Plane";
                    this.last_event_name = event_name;
                    this.last_event_value = format!(
                        "h:{:.0} s:{:.0}% l:{:.0}% a:{:.2}",
                        hsla.h * 360.0,
                        hsla.s * 100.0,
                        hsla.l * 100.0,
                        hsla.a
                    );
                    let hsv = this.hsv;

                    match this.mode {
                        PlaneMode::SvAtHue => {
                            this.slider.update(cx, |s, cx| s.set_value(hsv.h, cx))
                        }
                        PlaneMode::HsAtValue => this.slider.update(cx, |s, cx| {
                            s.set_delegate(
                                Box::new(
                                    ChannelDelegate::new(hsv, Hsv::VALUE.into())
                                        .expect("Failed to create Value ChannelDelegate"),
                                ),
                                cx,
                            )
                        }),
                        PlaneMode::HvAtSaturation => this.slider.update(cx, |s, cx| {
                            s.set_delegate(
                                Box::new(
                                    ChannelDelegate::new(hsv, Hsv::SATURATION.into())
                                        .expect("Failed to create Saturation ChannelDelegate"),
                                ),
                                cx,
                            )
                        }),
                    }

                    cx.notify();
                },
            ));

            _subscriptions.push(cx.subscribe(
                &slider_clone,
                move |this: &mut Self, _, event, cx| {
                    let (event_name, val) = match event {
                        ColorSliderEvent::Change(val) => ("Change", *val),
                        ColorSliderEvent::Release(val) => ("Release", *val),
                    };
                    match this.mode {
                        PlaneMode::SvAtHue => this.hsv.h = val,
                        PlaneMode::HsAtValue => this.hsv.v = val,
                        PlaneMode::HvAtSaturation => this.hsv.s = val,
                    }
                    let hsla = this.hsv.to_hsla_ext();
                    this.last_event_source = "Slider";
                    this.last_event_name = event_name;
                    this.last_event_value = format!(
                        "h:{:.0} s:{:.0}% l:{:.0}% a:{:.2}",
                        hsla.h * 360.0,
                        hsla.s * 100.0,
                        hsla.l * 100.0,
                        hsla.a
                    );
                    let hsv = this.hsv;
                    this.plane
                        .update(cx, |p, cx| p.set_hsv_components(hsv.h, hsv.s, hsv.v, cx));
                    cx.notify();
                },
            ));

            Self {
                label,
                hsv: initial_hsv,
                plane,
                slider,
                mode,
                plane_width,
                plane_height,
                clip_corner_radius: plane_corner_radius,
                show_checkerboard,
                thumb_position,
                last_event_source: "Plane",
                last_event_name: "Change",
                last_event_value: {
                    let hsla = initial_hsv.to_hsla_ext();
                    format!(
                        "h:{:.0} s:{:.0}% l:{:.0}% a:{:.2}",
                        hsla.h * 360.0,
                        hsla.s * 100.0,
                        hsla.l * 100.0,
                        hsla.a
                    )
                },
                _subscriptions,
            }
        })
    }

    fn set_plane_options(
        &mut self,
        thumb_position: FieldThumbPosition,
        corner_radius: f32,
        show_border: bool,
        cx: &mut Context<Self>,
    ) {
        self.thumb_position = thumb_position;
        self.clip_corner_radius = corner_radius;
        self.plane.update(cx, |plane, cx| {
            plane.set_thumb_position(thumb_position, cx);
            plane.set_corner_radius(px(corner_radius).into(), cx);
            plane.set_show_border(show_border, cx);
        });
        cx.notify();
    }
}

impl Render for ColorPlaneTestPicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let hsla_color = self.hsv.to_hsla_ext();
        let hsl_text = format!(
            "hsl({:.0},{:.0}%,{:.0}%)",
            hsla_color.h * 360.0,
            hsla_color.s * 100.0,
            hsla_color.l * 100.0
        );

        let plane_width = self.plane_width;
        let plane_height = self.plane_height;

        v_flex()
            .w(px(plane_width))
            .gap_2()
            .when_some(self.label.clone(), |this, label| {
                this.child(
                    div()
                        .text_size(px(COLOR_VALUE_FONT_SIZE))
                        .font_family(cx.theme().mono_font_family.clone())
                        .child(label),
                )
            })
            .child({
                let is_edge_to_edge = self.thumb_position == FieldThumbPosition::EdgeToEdge;

                let mut wrapper = div().w(px(plane_width)).h(px(plane_height)).flex_shrink_0();

                if is_edge_to_edge {
                    // EdgeToEdge: no clipping, thumb can extend beyond
                    wrapper = wrapper.child(self.plane.clone());
                } else {
                    // Other modes: clip overflow and round corners
                    wrapper = wrapper
                        .rounded(px(self.clip_corner_radius))
                        .overflow_hidden()
                        .child(self.plane.clone());
                }

                wrapper
            })
            .child(self.slider.clone())
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .rounded(px(4.0))
                    .bg(cx.theme().background.opacity(0.72))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_size(px(COLOR_VALUE_FONT_SIZE))
                            .font_family(cx.theme().mono_font_family.clone())
                            .child(format!(
                                "{} {}",
                                self.last_event_source, self.last_event_name
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(COLOR_VALUE_FONT_SIZE))
                            .font_family(cx.theme().mono_font_family.clone())
                            .child(self.last_event_value.clone()),
                    ),
            )
            .when(self.show_checkerboard, |this| {
                this.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(plane_width / 2.0))
                                .h(px(24.0))
                                .rounded(px(2.0))
                                .overflow_hidden()
                                .child(
                                    Checkerboard::new(cx.theme().is_dark())
                                        .child(div().size_full().bg(hsla_color)),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(COLOR_VALUE_FONT_SIZE))
                                .font_family(cx.theme().mono_font_family.clone())
                                .child(hsl_text),
                        ),
                )
            })
    }
}

pub struct StoryColorPlanePanel {
    // Variant tests
    test_a: Entity<ColorPlaneTestPicker>,
    test_b: Entity<ColorPlaneTestPicker>,
    test_c: Entity<ColorPlaneTestPicker>,
    test_size_xsmall: Entity<ColorPlaneTestPicker>,
    test_size_small: Entity<ColorPlaneTestPicker>,
    test_size_medium: Entity<ColorPlaneTestPicker>,
    test_size_large: Entity<ColorPlaneTestPicker>,
    border_examples_edge_to_edge: bool,
    border_examples_square_corners: bool,
    border_examples_no_border: bool,

    _subscriptions: Vec<Subscription>,
}

impl StoryColorPlanePanel {
    fn apply_border_example_controls(&mut self, cx: &mut Context<Self>) {
        let thumb_position = if self.border_examples_edge_to_edge {
            FieldThumbPosition::EdgeToEdge
        } else {
            FieldThumbPosition::EdgeToEdgeClipped
        };
        let corner_radius = if self.border_examples_square_corners {
            0.0
        } else {
            12.0
        };
        let show_border = !self.border_examples_no_border;

        self.test_a.update(cx, |picker, cx| {
            picker.set_plane_options(thumb_position, corner_radius, show_border, cx)
        });
        self.test_b.update(cx, |picker, cx| {
            picker.set_plane_options(thumb_position, corner_radius, show_border, cx)
        });
        self.test_c.update(cx, |picker, cx| {
            picker.set_plane_options(thumb_position, corner_radius, show_border, cx)
        });
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hsv = Hsv::from_hsla_ext(gpui::red());
        let hsv_size_xsmall = Hsv::from_hsla_ext(gpui::hsla(57.0 / 360.0, 1.0, 0.36, 1.0));
        let hsv_size_small = Hsv::from_hsla_ext(gpui::hsla(121.0 / 360.0, 1.0, 0.33, 1.0));
        let hsv_size_medium = Hsv::from_hsla_ext(gpui::hsla(230.0 / 360.0, 1.0, 0.33, 1.0));
        let hsv_size_large = Hsv::from_hsla_ext(gpui::hsla(303.0 / 360.0, 1.0, 0.31, 1.0));
        const PLANE_SIZE: f32 = PLANE_DIMENSION_MEDIUM;

        let test_a = ColorPlaneTestPicker::new(
            cx,
            Some("SaturationValue"),
            PlaneMode::SvAtHue,
            "test_a",
            hsv,
            12.0,
            0.0,
            true,
            PLANE_SIZE,
            PLANE_SIZE,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let test_b = ColorPlaneTestPicker::new(
            cx,
            Some("HueSaturation"),
            PlaneMode::HsAtValue,
            "test_b",
            hsv,
            12.0,
            0.0,
            true,
            PLANE_SIZE,
            PLANE_SIZE,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let test_c = ColorPlaneTestPicker::new(
            cx,
            Some("HueValue"),
            PlaneMode::HvAtSaturation,
            "test_c",
            hsv,
            12.0,
            0.0,
            true,
            PLANE_SIZE,
            PLANE_SIZE,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let test_size_xsmall = ColorPlaneTestPicker::new(
            cx,
            Some("HueValue\n(border, clipped, xsmall)"),
            PlaneMode::HvAtSaturation,
            "test_size_xsmall",
            hsv_size_xsmall,
            12.0,
            0.0,
            true,
            PLANE_DIMENSION_XSMALL,
            PLANE_DIMENSION_XSMALL,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let test_size_small = ColorPlaneTestPicker::new(
            cx,
            Some("HueValue\n(border, clipped, small)"),
            PlaneMode::HvAtSaturation,
            "test_size_small",
            hsv_size_small,
            12.0,
            0.0,
            true,
            PLANE_DIMENSION_SMALL,
            PLANE_DIMENSION_SMALL,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let test_size_medium = ColorPlaneTestPicker::new(
            cx,
            Some("HueValue\n(border, clipped, medium)"),
            PlaneMode::HvAtSaturation,
            "test_size_medium",
            hsv_size_medium,
            12.0,
            0.0,
            true,
            PLANE_DIMENSION_MEDIUM,
            PLANE_DIMENSION_MEDIUM,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let test_size_large = ColorPlaneTestPicker::new(
            cx,
            Some("HueValue\n(border, clipped, large)"),
            PlaneMode::HvAtSaturation,
            "test_size_large",
            hsv_size_large,
            12.0,
            0.0,
            true,
            PLANE_DIMENSION_LARGE,
            PLANE_DIMENSION_LARGE,
            true,
            FieldThumbPosition::EdgeToEdgeClipped,
        );

        let mut panel = Self {
            test_a,
            test_b,
            test_c,
            test_size_xsmall,
            test_size_small,
            test_size_medium,
            test_size_large,
            border_examples_edge_to_edge: false,
            border_examples_square_corners: false,
            border_examples_no_border: false,
            _subscriptions: vec![],
        };
        panel.apply_border_example_controls(cx);
        panel
    }
}

impl Render for StoryColorPlanePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let plane_tests_section = section("Plane Border Examples").max_w_full().child(
            v_flex().w_full().gap_6().child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .items_start()
                    .gap(px(32.0))
                    .child(
                        h_flex()
                            .items_start()
                            .gap(px(48.0))
                            .child(self.test_a.clone())
                            .child(self.test_b.clone())
                            .child(self.test_c.clone()),
                    )
                    .child(
                        v_flex()
                            .w(px(220.0))
                            .gap_2()
                            .p_3()
                            .rounded(px(6.0))
                            .bg(cx.theme().background.opacity(0.72))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .text_size(px(COLOR_VALUE_FONT_SIZE))
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child("Controls"),
                            )
                            .child(
                                Switch::new("plane-border-edge-to-edge")
                                    .checked(self.border_examples_edge_to_edge)
                                    .label("Edge-to-edge")
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.border_examples_edge_to_edge = *checked;
                                        this.apply_border_example_controls(cx);
                                    })),
                            )
                            .child(
                                Switch::new("plane-border-square-corners")
                                    .checked(self.border_examples_square_corners)
                                    .label("Square corners")
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.border_examples_square_corners = *checked;
                                        this.apply_border_example_controls(cx);
                                    })),
                            )
                            .child(
                                Switch::new("plane-border-no-border")
                                    .checked(self.border_examples_no_border)
                                    .label("No border")
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.border_examples_no_border = *checked;
                                        this.apply_border_example_controls(cx);
                                    })),
                            ),
                    ),
            ),
        );

        let plane_size_section = section("Plane Size Examples").max_w_full().child(
            h_flex()
                .w_full()
                .justify_center()
                .items_start()
                .gap(px(48.0))
                .child(self.test_size_xsmall.clone())
                .child(self.test_size_small.clone())
                .child(self.test_size_medium.clone())
                .child(self.test_size_large.clone()),
        );

        v_flex()
            .size_full()
            .gap_6()
            .child(plane_size_section)
            .child(plane_tests_section)
    }
}
}
