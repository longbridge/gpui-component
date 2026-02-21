use super::super::color_field::{
    ColorField, ColorFieldEvent, ColorFieldModel2D, ColorFieldState, TriangleDomain,
};
use super::super::color_ring::{ColorRing, ColorRingEvent, ColorRingState, HueRingDelegate};
use super::super::color_slider::sizing as slider_constants;
use super::super::color_slider::{ColorSlider, ColorSliderEvent, ColorSliderState};
use super::super::color_spec::Hsv;
use super::super::delegates::ChannelDelegate;
use super::super::mouse_behavior::MouseCursorDecision;
use super::color_text_label::ColorTextLabel;
use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Colorize as _, Sizable, Size};
use std::sync::Arc;

// Composition overview:
// - Single source of truth is `HueRingSvTrianglePickerState::hsv`.
// - Ring edits hue; triangle edits saturation/value; sliders mirror and edit all channels.
// - This file intentionally specializes `ColorFieldState` locally (domain + model)
//   instead of modifying shared `color_field` primitives.

/// State + wiring for the Photoshop-style hue ring + SV triangle composition.
pub struct HueRingSvTrianglePickerState {
    hsv: Hsv,
    color_ring: Entity<ColorRingState>,
    triangle_sv: Entity<ColorFieldState>,
    slider_h: Entity<ColorSliderState>,
    slider_s: Entity<ColorSliderState>,
    slider_b: Entity<ColorSliderState>,
    ring_interaction_active: bool,
    triangle_interaction_active: bool,
    _subscriptions: Vec<Subscription>,
}

impl HueRingSvTrianglePickerState {
    pub const RING_OUTER_SIZE_PX: f32 = 300.0;

    pub fn panel_width_px() -> f32 {
        Self::RING_OUTER_SIZE_PX + 56.0
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_hsv = Hsv::from_hsla_ext(
            Rgba {
                r: 214.0 / 255.0,
                g: 37.0 / 255.0,
                b: 162.0 / 255.0,
                a: 1.0,
            }
            .into(),
        );

        let color_ring = cx.new(|cx| {
            ColorRingState::hue(
                "composition_hue_ring_sv_triangle",
                initial_hsv.h,
                HueRingDelegate {
                    // Keep hue ring vivid regardless of current triangle selection.
                    saturation: 1.0,
                    lightness: 0.5,
                },
                cx,
            )
            .with_size(Size::Size(px(Self::RING_OUTER_SIZE_PX)))
            .ring_thickness_size(Size::Medium)
            .thumb_size(slider_constants::THUMB_SIZE_MEDIUM)
            .mouse_behavior_custom(|_| MouseCursorDecision::Passthrough)
        });

        let triangle_sv = cx.new(|_cx| {
            // Local `ColorField` specialization for this composition:
            // - custom right-pointing triangle domain
            // - custom HSV triangle model (`PhotoshopSvTriangleModel`) defined below
            let (white, black, hue) = sv_triangle_vertices();
            ColorFieldState::new(
                "composition_sv_triangle_plane",
                initial_hsv,
                Arc::new(TriangleDomain {
                    a: white,
                    b: black,
                    c: hue,
                }),
                Arc::new(PhotoshopSvTriangleModel),
            )
            .thumb_size(slider_constants::THUMB_SIZE_MEDIUM)
            .mouse_behavior_custom(|_| MouseCursorDecision::Passthrough)
            // Intentionally rasterized: this triangle model is composition-local and
            // looks smoother/closer to Photoshop with sampled fills than with vector tiling.
            .raster_image()
            .rounded(px(0.0))
            .no_border()
            .edge_to_edge()
        });

        let slider_h = cx.new(|cx| {
            let mut slider =
                ColorSliderState::hue("composition_hue_ring_sv_triangle_h", initial_hsv.h, cx)
                    .horizontal()
                    .rounded(px(0.0))
                    .thumb_small()
                    .thumb_square();
            slider.set_size(Size::Small, cx);
            slider
        });

        let slider_s = cx.new(|cx| {
            let mut slider = ColorSliderState::channel(
                "composition_hue_ring_sv_triangle_s",
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

        let slider_b = cx.new(|cx| {
            let mut slider = ColorSliderState::channel(
                "composition_hue_ring_sv_triangle_b",
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

        let mut _subscriptions = Vec::new();

        _subscriptions.push(cx.subscribe(&color_ring, |this, _, event, cx| {
            let hue = match event {
                ColorRingEvent::Change(value) => {
                    this.ring_interaction_active = true;
                    *value
                }
                ColorRingEvent::Release(value) => {
                    this.ring_interaction_active = false;
                    *value
                }
            };
            this.hsv.h = hue;
            this.sync_controls(cx);
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&triangle_sv, |this, _, event, cx| {
            let (saturation, value) = match event {
                ColorFieldEvent::Change(hsv) => {
                    this.triangle_interaction_active = true;
                    (hsv.s, hsv.v)
                }
                ColorFieldEvent::Release(hsv) => {
                    this.triangle_interaction_active = false;
                    (hsv.s, hsv.v)
                }
            };
            this.hsv.s = saturation;
            this.hsv.v = value;
            this.sync_controls(cx);
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&slider_h, |this, _, event, cx| {
            let hue = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            this.hsv.h = hue;
            this.sync_controls(cx);
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&slider_s, |this, _, event, cx| {
            let saturation = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            this.hsv.s = saturation;
            this.sync_controls(cx);
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&slider_b, |this, _, event, cx| {
            let value = match event {
                ColorSliderEvent::Change(value) | ColorSliderEvent::Release(value) => *value,
            };
            this.hsv.v = value;
            this.sync_controls(cx);
            cx.notify();
        }));

        let mut this = Self {
            hsv: initial_hsv,
            color_ring,
            triangle_sv,
            slider_h,
            slider_s,
            slider_b,
            ring_interaction_active: false,
            triangle_interaction_active: false,
            _subscriptions,
        };
        this.sync_controls(cx);
        this
    }

    // Push canonical HSV state to all child controls so they stay in lock-step.
    fn sync_controls(&mut self, cx: &mut Context<Self>) {
        let hsv = self.hsv;

        self.color_ring
            .update(cx, |ring, cx| ring.set_value(hsv.h, cx));

        self.triangle_sv.update(cx, |field, cx| {
            field.set_hsv_components(hsv.h, hsv.s, hsv.v, cx);
        });

        self.slider_h
            .update(cx, |slider, cx| slider.set_value(hsv.h, cx));

        self.slider_s.update(cx, |slider, cx| {
            slider.set_value(hsv.s, cx);
            slider.set_delegate(
                Box::new(
                    ChannelDelegate::new(hsv, Hsv::SATURATION.into())
                        .expect("Failed to create HSV Saturation ChannelDelegate"),
                ),
                cx,
            );
        });

        self.slider_b.update(cx, |slider, cx| {
            slider.set_value(hsv.v, cx);
            slider.set_delegate(
                Box::new(
                    ChannelDelegate::new(hsv, Hsv::VALUE.into())
                        .expect("Failed to create HSV Value ChannelDelegate"),
                ),
                cx,
            );
        });
    }
}

impl Render for HueRingSvTrianglePickerState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        HueRingSvTrianglePicker::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct HueRingSvTrianglePicker {
    state: Entity<HueRingSvTrianglePickerState>,
}

impl HueRingSvTrianglePicker {
    pub fn new(state: &Entity<HueRingSvTrianglePickerState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for HueRingSvTrianglePicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let picker_state = self.state.clone();

        // Layout:
        // 1) top rows: H/S/B sliders + textual values
        // 2) bottom: hue ring with centered triangle overlay sized to inner ring diameter
        let (
            hsv,
            ring_outer_size,
            plane_size,
            plane_half,
            color_ring,
            triangle_sv,
            slider_h,
            slider_s,
            slider_b,
            ring_color,
        ) = {
            let state = self.state.read(cx);
            let ring_state = state.color_ring.read(cx);
            let ring_outer_size = match ring_state.size {
                Size::XSmall => 140.0,
                Size::Small => 180.0,
                Size::Medium => 220.0,
                Size::Large => 280.0,
                Size::Size(px) => px.as_f32(),
            };
            let ring_inner_diameter =
                (ring_outer_size - 2.0 * ring_state.ring_thickness_px()).max(0.0);
            let plane_size = (ring_inner_diameter - 2.0).max(40.0);
            let plane_half = plane_size / 2.0;

            (
                state.hsv,
                ring_outer_size,
                plane_size,
                plane_half,
                state.color_ring.clone(),
                state.triangle_sv.clone(),
                state.slider_h.clone(),
                state.slider_s.clone(),
                state.slider_b.clone(),
                state.hsv.to_hsla_ext(),
            )
        };

        v_flex()
            .w(px(HueRingSvTrianglePickerState::panel_width_px()))
            .max_w_full()
            .items_center()
            .gap_3()
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .child(render_slider_row(
                        "H",
                        slider_h,
                        format!("{:.0}", hsv.h),
                        cx,
                    ))
                    .child(render_slider_row(
                        "S",
                        slider_s,
                        format!("{:.0}%", hsv.s * 100.0),
                        cx,
                    ))
                    .child(render_slider_row(
                        "B",
                        slider_b,
                        format!("{:.0}%", hsv.v * 100.0),
                        cx,
                    )),
            )
            .child(
                h_flex().w_full().justify_center().child(
                    v_flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .size(px(ring_outer_size))
                                .relative()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(ColorRing::new(&color_ring))
                                .child(
                                    div()
                                        .absolute()
                                        .top_1_2()
                                        .left_1_2()
                                        .mt(px(-plane_half))
                                        .ml(px(-plane_half))
                                        .size(px(plane_size))
                                        .child(ColorField::new(&triangle_sv)),
                                )
                                .child(
                                    canvas(
                                        |bounds, window, _| {
                                            window.insert_hitbox(bounds, HitboxBehavior::Normal)
                                        },
                                        {
                                            let picker_state = picker_state.clone();
                                            move |_, hitbox, window, cx| {
                                                let pointer = window.mouse_position();
                                                let state = picker_state.read(cx);
                                                let ring_contains =
                                                    state.color_ring.read(cx).contains_pointer(pointer);
                                                let triangle_contains =
                                                    state.triangle_sv.read(cx).contains_pointer(pointer);

                                                if state.ring_interaction_active
                                                    || state.triangle_interaction_active
                                                {
                                                    window.set_window_cursor_style(
                                                        CursorStyle::Crosshair,
                                                    );
                                                } else if ring_contains || triangle_contains {
                                                    window.set_cursor_style(
                                                        CursorStyle::Crosshair,
                                                        &hitbox,
                                                    );
                                                }
                                            }
                                        },
                                    )
                                    .absolute()
                                    .inset_0(),
                                ),
                        )
                        .child(render_color_swatch(ring_color, ring_outer_size, cx)),
                ),
            )
    }
}

#[derive(Clone, Copy, Debug, Default)]
// Composition-local model that maps triangle barycentric coordinates <-> HSV(S,V).
struct PhotoshopSvTriangleModel;

impl ColorFieldModel2D for PhotoshopSvTriangleModel {
    fn apply_uv(&self, hsv: &mut Hsv, uv: (f32, f32)) {
        let x = uv.0.clamp(0.0, 1.0);
        let y = uv.1.clamp(0.0, 1.0);
        let (white, black, hue) = sv_triangle_vertices();
        let (_, black_weight, hue_weight) = barycentric((x, y), white, black, hue);
        let value = (1.0 - black_weight.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let saturation = if value <= f32::EPSILON {
            0.0
        } else {
            (hue_weight.clamp(0.0, 1.0) / value).clamp(0.0, 1.0)
        };

        hsv.s = saturation;
        hsv.v = value;
    }

    fn uv_from_hsv(&self, hsv: &Hsv) -> (f32, f32) {
        let saturation = hsv.s.clamp(0.0, 1.0);
        let value = hsv.v.clamp(0.0, 1.0);
        let hue_weight = (saturation * value).clamp(0.0, 1.0);
        let black_weight = (1.0 - value).clamp(0.0, 1.0);
        let white_weight = (value - hue_weight).clamp(0.0, 1.0);
        let (white, black, hue) = sv_triangle_vertices();

        (
            (white.0 * white_weight + black.0 * black_weight + hue.0 * hue_weight).clamp(0.0, 1.0),
            (white.1 * white_weight + black.1 * black_weight + hue.1 * hue_weight).clamp(0.0, 1.0),
        )
    }

    fn color_at_uv(&self, hsv: &Hsv, uv: (f32, f32)) -> Hsla {
        let x = uv.0.clamp(0.0, 1.0);
        let y = uv.1.clamp(0.0, 1.0);
        let (white, black, hue) = sv_triangle_vertices();
        let (_, black_weight, hue_weight) = barycentric((x, y), white, black, hue);
        let value = (1.0 - black_weight.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let saturation = if value <= f32::EPSILON {
            0.0
        } else {
            (hue_weight.clamp(0.0, 1.0) / value).clamp(0.0, 1.0)
        };

        Hsv {
            h: hsv.h,
            s: saturation,
            v: value,
            a: 1.0,
        }
        .to_hsla_ext()
    }
}

// Triangle vertices in normalized UV coordinates.
// white=(a), black=(b), hue=(c)
// Update these three points if you need to tweak geometry/fit.
fn sv_triangle_vertices() -> ((f32, f32), (f32, f32), (f32, f32)) {
    ((0.25, 0.066_987_3), (0.25, 0.933_012_7), (1.0, 0.5))
}

// Returns barycentric weights of `point` within triangle (a,b,c).
fn barycentric(point: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> (f32, f32, f32) {
    let denom = (b.1 - c.1) * (a.0 - c.0) + (c.0 - b.0) * (a.1 - c.1);
    if denom.abs() <= f32::EPSILON {
        return (0.0, 0.0, 0.0);
    }

    let wa = ((b.1 - c.1) * (point.0 - c.0) + (c.0 - b.0) * (point.1 - c.1)) / denom;
    let wb = ((c.1 - a.1) * (point.0 - c.0) + (a.0 - c.0) * (point.1 - c.1)) / denom;
    let wc = 1.0 - wa - wb;
    (wa, wb, wc)
}

fn render_slider_row(
    label: &'static str,
    slider: Entity<ColorSliderState>,
    value: String,
    cx: &mut App,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(px(12.0))
                .text_size(px(9.0))
                .font_family(cx.theme().mono_font_family.clone())
                .child(label),
        )
        .child(div().flex_1().child(ColorSlider::new(&slider)))
        .child(
            div()
                .w(px(44.0))
                .text_right()
                .text_size(px(10.0))
                .font_family(cx.theme().mono_font_family.clone())
                .text_color(cx.theme().muted_foreground)
                .child(value),
        )
}

fn render_color_swatch(color: gpui::Hsla, width: f32, cx: &mut App) -> impl IntoElement {
    v_flex()
        .w(px(width))
        .items_center()
        .gap_1()
        .child(
            div()
                .w(px(width))
                .h(px(40.0))
                .rounded_md()
                .bg(color)
                .border_1()
                .border_color(cx.theme().border),
        )
        .child(
            ColorTextLabel::new("hue-ring-sv-triangle-copy", color.to_hex().to_uppercase().into())
                .width_px(width),
        )
}
