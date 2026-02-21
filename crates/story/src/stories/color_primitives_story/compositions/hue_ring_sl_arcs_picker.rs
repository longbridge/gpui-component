use super::super::color_arc::{
    raster::RasterArcDelegate, ColorArc, ColorArcEvent, ColorArcRenderer, ColorArcState,
};
use super::super::color_ring::{ColorRing, ColorRingEvent, ColorRingState, HueRingDelegate};
use super::color_text_label::ColorTextLabel;
use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme, Colorize as _, Sizable};

pub struct HueRingSlArcsPickerState {
    saturation_arc: Entity<ColorArcState>,
    lightness_arc: Entity<ColorArcState>,
    hue_ring: Entity<ColorRingState>,
    hue_degrees: f32,
    saturation: f32,
    lightness: f32,
    swatch_color: Hsla,
    _subscriptions: Vec<Subscription>,
}

impl HueRingSlArcsPickerState {
    pub const OUTER_SIZE_PX: f32 = 300.0;
    pub const TRACK_WIDTH_PX: f32 = 20.0;
    pub const ARC_RING_GAP_PX: f32 = 5.0;
    pub const RING_SWATCH_GAP_PX: f32 = 14.0;
    pub const OUTER_PADDING_PX: f32 = 12.0;
    pub const BORDER_GAP_PX: f32 = 14.0;

    const ARC_GAP_DEGREES: f32 = 8.0;
    const ARC_ROTATION_DEGREES: f32 = 90.0;
    const ARC_SWEEP_DEGREES: f32 = 180.0 - Self::ARC_GAP_DEGREES;
    const ARC_HORIZONTAL_OFFSET_PX: f32 = 5.0;

    fn ring_outer_radius_px() -> f32 {
        (Self::OUTER_SIZE_PX * 0.5 - Self::TRACK_WIDTH_PX - Self::ARC_RING_GAP_PX).max(0.0)
    }

    fn frame_size_px() -> f32 {
        Self::OUTER_SIZE_PX + Self::ARC_HORIZONTAL_OFFSET_PX * 2.0
    }

    fn border_size_px() -> f32 {
        Self::frame_size_px() + Self::BORDER_GAP_PX * 2.0
    }

    fn canvas_size_px() -> f32 {
        Self::border_size_px() + Self::OUTER_PADDING_PX * 2.0
    }

    pub fn panel_width_px() -> f32 {
        Self::canvas_size_px() + 40.0
    }

    fn ring_size_px() -> f32 {
        Self::ring_outer_radius_px() * 2.0
    }

    fn swatch_size_px() -> f32 {
        let swatch_radius =
            (Self::ring_outer_radius_px() - Self::TRACK_WIDTH_PX - Self::RING_SWATCH_GAP_PX)
                .max(0.0);
        swatch_radius * 2.0
    }

    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let init_color: Hsla = Rgba {
            r: 229.0 / 255.0,
            g: 62.0 / 255.0,
            b: 18.0 / 255.0,
            a: 1.0,
        }
        .into();
        let hue_degrees = init_color.h * 360.0;
        let saturation = init_color.s;
        let lightness = init_color.l;

        let saturation_arc = cx.new(|cx| {
            ColorArcState::saturation_with_renderer(
                "composition_saturation_arc",
                saturation,
                hue_degrees,
                1.0,
                ColorArcRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Size(px(Self::OUTER_SIZE_PX)))
            .start_degrees(90.0 + Self::ARC_ROTATION_DEGREES + Self::ARC_GAP_DEGREES * 0.5)
            .sweep_degrees(Self::ARC_SWEEP_DEGREES)
            .arc_thickness(Self::TRACK_WIDTH_PX)
            .thumb_size(Self::TRACK_WIDTH_PX)
        });

        let lightness_arc = cx.new(|cx| {
            ColorArcState::lightness_with_renderer(
                "composition_lightness_arc",
                lightness,
                hue_degrees,
                saturation,
                ColorArcRenderer::Raster,
                cx,
            )
            .with_size(gpui_component::Size::Size(px(Self::OUTER_SIZE_PX)))
            .start_degrees(270.0 + Self::ARC_ROTATION_DEGREES + Self::ARC_GAP_DEGREES * 0.5)
            .sweep_degrees(Self::ARC_SWEEP_DEGREES)
            .arc_thickness(Self::TRACK_WIDTH_PX)
            .thumb_size(Self::TRACK_WIDTH_PX)
        });

        let hue_ring = cx.new(|cx| {
            ColorRingState::hue(
                "composition_hue_ring",
                hue_degrees,
                HueRingDelegate {
                    saturation,
                    lightness,
                },
                cx,
            )
            .with_size(gpui_component::Size::Size(px(Self::ring_size_px())))
            .ring_thickness(Self::TRACK_WIDTH_PX)
            .thumb_size(Self::TRACK_WIDTH_PX)
        });

        let mut _subscriptions = Vec::new();

        _subscriptions.push(cx.subscribe(&hue_ring, |this, _, event, cx| {
            let hue = match event {
                ColorRingEvent::Change(value) | ColorRingEvent::Release(value) => *value,
            };
            this.hue_degrees = hue;
            this.sync(cx);
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&saturation_arc, |this, _, event, cx| {
            let saturation = match event {
                ColorArcEvent::Change(value) | ColorArcEvent::Release(value) => *value,
            };
            this.saturation = saturation.clamp(0.0, 1.0);
            this.sync(cx);
            cx.notify();
        }));

        _subscriptions.push(cx.subscribe(&lightness_arc, |this, _, event, cx| {
            let lightness = match event {
                ColorArcEvent::Change(value) | ColorArcEvent::Release(value) => *value,
            };
            this.lightness = lightness.clamp(0.0, 1.0);
            this.sync(cx);
            cx.notify();
        }));

        let mut this = Self {
            saturation_arc,
            lightness_arc,
            hue_ring,
            hue_degrees,
            saturation,
            lightness,
            swatch_color: init_color,
            _subscriptions,
        };
        this.sync(cx);
        this
    }

    fn sync(&mut self, cx: &mut Context<Self>) {
        self.swatch_color = hsla(
            (self.hue_degrees / 360.0).rem_euclid(1.0),
            self.saturation.clamp(0.0, 1.0),
            self.lightness.clamp(0.0, 1.0),
            1.0,
        );

        let hue_degrees = self.hue_degrees;
        let saturation = self.saturation;
        let lightness = self.lightness;

        self.hue_ring.update(cx, |ring, cx| {
            ring.set_value(hue_degrees, cx);
            ring.set_delegate(
                Box::new(HueRingDelegate {
                    saturation,
                    lightness,
                }),
                cx,
            );
        });

        self.saturation_arc.update(cx, |arc, cx| {
            arc.set_value(saturation, cx);
            arc.set_delegate(
                Box::new(RasterArcDelegate::saturation(hue_degrees, 1.0)),
                cx,
            );
        });

        self.lightness_arc.update(cx, |arc, cx| {
            arc.set_value(lightness, cx);
            arc.set_delegate(
                Box::new(RasterArcDelegate::lightness(hue_degrees, saturation)),
                cx,
            );
        });
    }
}

impl Render for HueRingSlArcsPickerState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        HueRingSlArcsPicker::new(&cx.entity())
    }
}

#[derive(IntoElement)]
pub struct HueRingSlArcsPicker {
    state: Entity<HueRingSlArcsPickerState>,
}

impl HueRingSlArcsPicker {
    pub fn new(state: &Entity<HueRingSlArcsPickerState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for HueRingSlArcsPicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        const COLOR_LABEL_GAP_PX: f32 = 8.0;
        let (saturation_arc, lightness_arc, hue_ring, swatch_color) = {
            let state = self.state.read(cx);
            (
                state.saturation_arc.clone(),
                state.lightness_arc.clone(),
                state.hue_ring.clone(),
                state.swatch_color,
            )
        };

        let ring_size = HueRingSlArcsPickerState::ring_size_px();
        let swatch_size = HueRingSlArcsPickerState::swatch_size_px();
        let group_width = HueRingSlArcsPickerState::frame_size_px();
        let border_size = HueRingSlArcsPickerState::border_size_px();
        let canvas_size = HueRingSlArcsPickerState::canvas_size_px();
        let arc_offset = HueRingSlArcsPickerState::ARC_HORIZONTAL_OFFSET_PX;

        let group_left = (canvas_size - group_width) * 0.5;
        let group_top = (canvas_size - HueRingSlArcsPickerState::OUTER_SIZE_PX) * 0.5;
        let border_left = (canvas_size - border_size) * 0.5;
        let border_top = (canvas_size - border_size) * 0.5;
        let color_label = swatch_color.to_hex().to_uppercase();

        let ring_offset = (HueRingSlArcsPickerState::OUTER_SIZE_PX - ring_size) * 0.5;
        let swatch_offset = (HueRingSlArcsPickerState::OUTER_SIZE_PX - swatch_size) * 0.5;

        h_flex().w_full().justify_center().child(
            div()
                .w(px(HueRingSlArcsPickerState::panel_width_px()))
                .max_w_full()
                .child(
                    v_flex()
                        .items_center()
                        .gap(px(COLOR_LABEL_GAP_PX))
                        .child(
                            div()
                                .relative()
                                .size(px(canvas_size))
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(border_left))
                                        .top(px(border_top))
                                        .size(px(border_size))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(cx.theme().border),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(group_left))
                                        .top(px(group_top))
                                        .size(px(HueRingSlArcsPickerState::OUTER_SIZE_PX))
                                        .child(ColorArc::new(&saturation_arc)),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(group_left + arc_offset * 2.0))
                                        .top(px(group_top))
                                        .size(px(HueRingSlArcsPickerState::OUTER_SIZE_PX))
                                        .child(ColorArc::new(&lightness_arc)),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(group_left + arc_offset + ring_offset))
                                        .top(px(group_top + ring_offset))
                                        .size(px(ring_size))
                                        .child(ColorRing::new(&hue_ring)),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(group_left + arc_offset + swatch_offset))
                                        .top(px(group_top + swatch_offset))
                                        .size(px(swatch_size))
                                        .rounded_full()
                                        .bg(swatch_color)
                                        .border_1()
                                        .border_color(cx.theme().border),
                                ),
                        )
                        .child(
                            ColorTextLabel::new("hue-ring-sl-arcs-copy", color_label.into())
                                .width_px(HueRingSlArcsPickerState::OUTER_SIZE_PX),
                        ),
                ),
        )
    }
}
