use super::super::color_field::{ColorFieldEvent, ColorFieldState};
use super::super::color_ring::{ColorRingEvent, ColorRingState, HueRingDelegate};
use super::super::color_spec::{ColorSpecification, Hsv};
use super::color_text_label::ColorTextLabel;
use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme, Colorize, PixelsExt as _, Sizable};
use std::f32::consts::SQRT_2;

pub struct HueRingSvSquarePickerState {
    color_ring: Entity<ColorRingState>,
    plane_sv: Entity<ColorFieldState>,
    swatch_hsv: Hsv,
    ring_color: Hsla,
    _subscriptions: Vec<Subscription>,
}

impl HueRingSvSquarePickerState {
    pub const RING_OUTER_SIZE_PX: f32 = 300.0; // 332.0;

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let init_hsla: Hsla = Rgba {
            r: 18.0 / 255.0,
            g: 48.0 / 255.0,
            b: 154.0 / 255.0,
            a: 1.0,
        }
        .into();
        let init_hsv = Hsv::from_hsla(init_hsla);
        let color_ring = cx.new(|cx| {
            ColorRingState::hue(
                "composition_color_ring",
                init_hsv.h,
                HueRingDelegate {
                    // Keep ring rendering vivid regardless of initial square/swatch color.
                    saturation: 1.0,
                    lightness: 0.5,
                },
                cx,
            )
            .with_size(gpui_component::Size::Size(px(Self::RING_OUTER_SIZE_PX)))
            .ring_thickness_size(gpui_component::Size::Medium)
            .thumb_size(16.0)
        });

        let plane_sv = cx.new(|_cx| {
            ColorFieldState::saturation_value("composition_sv_plane", init_hsv, 16.0)
                .raster_image()
                .rounded(px(0.0))
                .no_border()
                .edge_to_edge()
        });

        let mut _subscriptions = Vec::new();

        _subscriptions.push(cx.subscribe(&color_ring, |this, _, event, cx| {
            let hue = match event {
                ColorRingEvent::Change(value) | ColorRingEvent::Release(value) => *value,
            };
            this.swatch_hsv.h = hue;
            let saturation = this.swatch_hsv.s;
            let value = this.swatch_hsv.v;
            this.plane_sv.update(cx, |plane, cx| {
                plane.set_hsv_components(hue, saturation, value, cx);
            });
            this.ring_color = this.swatch_hsv.to_hsla();
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&plane_sv, |this, _, event, cx| {
            let (saturation, value) = match event {
                ColorFieldEvent::Change(hsv) | ColorFieldEvent::Release(hsv) => (hsv.s, hsv.v),
            };
            this.swatch_hsv.s = saturation;
            this.swatch_hsv.v = value;
            this.ring_color = this.swatch_hsv.to_hsla();
            cx.notify();
        }));

        Self {
            color_ring,
            plane_sv,
            swatch_hsv: init_hsv,
            ring_color: init_hsv.to_hsla(),
            _subscriptions,
        }
    }
}

impl Render for HueRingSvSquarePickerState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        HueRingSvSquarePicker::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct HueRingSvSquarePicker {
    state: Entity<HueRingSvSquarePickerState>,
}

impl HueRingSvSquarePicker {
    pub fn new(state: &Entity<HueRingSvSquarePickerState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for HueRingSvSquarePicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (ring_outer_size, plane_half, plane_size, color_ring, plane_sv, ring_color) = {
            let state = self.state.read(cx);
            let ring_state = state.color_ring.read(cx);
            let ring_outer_size = match ring_state.size {
                gpui_component::Size::XSmall => 140.0,
                gpui_component::Size::Small => 180.0,
                gpui_component::Size::Medium => 220.0,
                gpui_component::Size::Large => 280.0,
                gpui_component::Size::Size(px) => px.as_f32(),
            };
            let ring_inner_diameter =
                (ring_outer_size - 2.0 * ring_state.ring_thickness_px()).max(0.0);
            // Largest axis-aligned square that fits in the inner circle, with a small visual margin.
            let plane_size = (ring_inner_diameter / SQRT_2 - 8.0).max(40.0);
            let plane_half = plane_size / 2.0;
            (
                ring_outer_size,
                plane_half,
                plane_size,
                state.color_ring.clone(),
                state.plane_sv.clone(),
                state.ring_color,
            )
        };

        h_flex().w_full().justify_center().child(
            div().w(px(ring_outer_size + 40.0)).max_w_full().child(
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
                            .child(color_ring)
                            .child(
                                div()
                                    .absolute()
                                    .top_1_2()
                                    .left_1_2()
                                    .mt(px(-plane_half))
                                    .ml(px(-plane_half))
                                    .size(px(plane_size))
                                    .child(plane_sv),
                            ),
                    )
                    .child(render_color_swatch(ring_color, ring_outer_size, cx)),
            ),
        )
    }
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
            ColorTextLabel::new("hue-ring-sv-square-copy", color.to_hex().to_uppercase().into())
                .width_px(width),
        )
}
