#![allow(dead_code)]

use super::arc::{ColorArcDelegate, ColorArcState};
use crate::stories::color_primitives_story::color_slider::color_spec::Hsv;
use gpui::{prelude::*, *};
use gpui_component::{
    plot::shape::{Arc, ArcData},
    ActiveTheme as _, PixelsExt as _,
};
use std::f32::consts::TAU;

fn normalize_hue_degrees(hue: f32) -> f32 {
    hue.rem_euclid(360.0)
}

fn segment_count(size: gpui_component::Size, sweep_turns: f32) -> usize {
    let full_ring_segments = match size {
        gpui_component::Size::XSmall | gpui_component::Size::Small => 180,
        gpui_component::Size::Medium => 360,
        gpui_component::Size::Large => 720,
        gpui_component::Size::Size(px) => {
            if px.as_f32() < 220.0 {
                180
            } else if px.as_f32() < 280.0 {
                360
            } else {
                720
            }
        }
    };

    ((full_ring_segments as f32 * sweep_turns).round() as usize).max(2)
}

fn logical_position(reversed: bool, position: f32) -> f32 {
    if reversed { 1.0 - position } else { position }.clamp(0.0, 1.0)
}

fn cap_center(bounds: Bounds<Pixels>, track_radius: f32, turn: f32) -> (f32, f32) {
    let theta = (turn.rem_euclid(1.0) - 0.25) * TAU;
    let center_x = bounds.origin.x.as_f32() + bounds.size.width.as_f32() / 2.0;
    let center_y = bounds.origin.y.as_f32() + bounds.size.height.as_f32() / 2.0;
    (
        center_x + track_radius * theta.cos(),
        center_y + track_radius * theta.sin(),
    )
}

fn paint_round_cap(window: &mut Window, center: (f32, f32), diameter: f32, color: Hsla) {
    let radius = diameter * 0.5;
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(px(center.0 - radius), px(center.1 - radius)),
            size: size(px(diameter), px(diameter)),
        },
        corner_radii: Corners::all(px(radius)),
        background: color.into(),
        border_widths: Edges::default(),
        border_color: transparent_black(),
        border_style: BorderStyle::default(),
    });
}

fn paint_arc_end_caps(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    start_turn: f32,
    sweep_turns: f32,
    arc_thickness: f32,
    outer_radius: Option<f32>,
    start_color: Hsla,
    end_color: Hsla,
) {
    if sweep_turns <= f32::EPSILON || sweep_turns >= 0.9999 || arc_thickness <= 0.0 {
        return;
    }

    let radius = outer_radius.unwrap_or(bounds.size.width.min(bounds.size.height).as_f32() * 0.5);
    let track_radius = (radius - arc_thickness * 0.5).max(0.0);
    let start_center = cap_center(bounds, track_radius, start_turn);
    let end_center = cap_center(bounds, track_radius, start_turn + sweep_turns);

    paint_round_cap(window, start_center, arc_thickness, start_color);
    paint_round_cap(window, end_center, arc_thickness, end_color);
}

fn paint_arc_border_underlay(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    start_turn: f32,
    sweep_turns: f32,
    arc_thickness: f32,
    border_color: Hsla,
) {
    if sweep_turns <= f32::EPSILON || arc_thickness <= 0.0 {
        return;
    }

    let radius = bounds.size.width.min(bounds.size.height).as_f32() * 0.5;
    if radius <= 0.0 {
        return;
    }

    let inner_radius = (radius - arc_thickness).max(0.0);
    let start_angle = start_turn * TAU;
    let end_angle = (start_turn + sweep_turns) * TAU;
    let arc_data = ArcData {
        data: &(),
        index: 0,
        value: 1.0,
        start_angle,
        end_angle,
        pad_angle: 0.0,
    };

    let arc_shape = Arc::new().inner_radius(inner_radius).outer_radius(radius);
    arc_shape.paint(&arc_data, border_color, None, None, &bounds, window);

    if sweep_turns < 0.9999 && arc_thickness > 0.0 {
        paint_arc_end_caps(
            window,
            bounds,
            start_turn,
            sweep_turns,
            arc_thickness,
            Some(radius),
            border_color,
            border_color,
        );
    }
}

pub struct HueArcDelegate {
    pub saturation: f32,
    pub lightness: f32,
}

impl ColorArcDelegate for HueArcDelegate {
    fn style_background(
        &self,
        arc: &ColorArcState,
        container: Div,
        _window: &mut Window,
        _cx: &App,
    ) -> Div {
        let saturation = self.saturation.clamp(0.0, 1.0);
        let lightness = self.lightness.clamp(0.0, 1.0);
        let start_turn = arc.start_turns();
        let sweep_turns = arc.sweep_turns();
        let reversed = arc.reversed;
        let border_color = _cx.theme().border;
        if sweep_turns <= f32::EPSILON {
            return container;
        }

        let segments = segment_count(arc.size, sweep_turns);
        let arc_thickness = arc.arc_thickness_px();
        let border_width = 1.6;
        let fill_thickness = (arc_thickness - border_width * 2.0).max(0.0);

        container.child(
            canvas(
                move |bounds, _, _| bounds,
                move |bounds, _prepaint, window, _| {
                    paint_arc_border_underlay(
                        window,
                        bounds,
                        start_turn,
                        sweep_turns,
                        arc_thickness,
                        border_color,
                    );
                    if fill_thickness <= 0.0 {
                        return;
                    }

                    let radius = bounds.size.width.min(bounds.size.height).as_f32() * 0.5;
                    let outer_fill_radius = (radius - border_width).max(0.0);
                    let inner_fill_radius = (outer_fill_radius - fill_thickness).max(0.0);
                    let arc_shape = Arc::new()
                        .inner_radius(inner_fill_radius)
                        .outer_radius(outer_fill_radius);
                    let step_turns = sweep_turns / segments as f32;
                    let overlap = (step_turns * TAU) * 0.35;

                    for i in 0..segments {
                        let t0 = i as f32 / segments as f32;
                        let t1 = (i + 1) as f32 / segments as f32;
                        let t_mid = (t0 + t1) * 0.5;
                        let logical = logical_position(reversed, t_mid);
                        let color = hsla(logical, saturation, lightness, 1.0);

                        let start_angle = (start_turn + t0 * sweep_turns) * TAU - overlap;
                        let end_angle = (start_turn + t1 * sweep_turns) * TAU + overlap;

                        arc_shape.paint(
                            &ArcData {
                                data: &(),
                                index: i,
                                value: 1.0,
                                start_angle,
                                end_angle,
                                pad_angle: 0.0,
                            },
                            color,
                            None,
                            None,
                            &bounds,
                            window,
                        );
                    }

                    paint_arc_end_caps(
                        window,
                        bounds,
                        start_turn,
                        sweep_turns,
                        fill_thickness,
                        Some(outer_fill_radius),
                        hsla(logical_position(reversed, 0.0), saturation, lightness, 1.0),
                        hsla(logical_position(reversed, 1.0), saturation, lightness, 1.0),
                    );
                },
            )
            .absolute()
            .inset_0(),
        )
    }

    fn get_color_at_position(&self, arc: &ColorArcState, position: f32) -> Hsla {
        let logical = logical_position(arc.reversed, position);
        hsla(
            logical,
            self.saturation.clamp(0.0, 1.0),
            self.lightness.clamp(0.0, 1.0),
            1.0,
        )
    }
}

pub struct SaturationArcDelegate {
    pub hue: f32,
    pub hsv_value: f32,
}

impl ColorArcDelegate for SaturationArcDelegate {
    fn style_background(
        &self,
        arc: &ColorArcState,
        container: Div,
        _window: &mut Window,
        _cx: &App,
    ) -> Div {
        let hue = normalize_hue_degrees(self.hue);
        let hsv_value = self.hsv_value.clamp(0.0, 1.0);
        let start_turn = arc.start_turns();
        let sweep_turns = arc.sweep_turns();
        let reversed = arc.reversed;
        let border_color = _cx.theme().border;
        if sweep_turns <= f32::EPSILON {
            return container;
        }

        let segments = segment_count(arc.size, sweep_turns);
        let arc_thickness = arc.arc_thickness_px();
        let border_width = 1.6;
        let fill_thickness = (arc_thickness - border_width * 2.0).max(0.0);

        container.child(
            canvas(
                move |bounds, _, _| bounds,
                move |bounds, _prepaint, window, _| {
                    paint_arc_border_underlay(
                        window,
                        bounds,
                        start_turn,
                        sweep_turns,
                        arc_thickness,
                        border_color,
                    );
                    if fill_thickness <= 0.0 {
                        return;
                    }

                    let radius = bounds.size.width.min(bounds.size.height).as_f32() * 0.5;
                    let outer_fill_radius = (radius - border_width).max(0.0);
                    let inner_fill_radius = (outer_fill_radius - fill_thickness).max(0.0);
                    let arc_shape = Arc::new()
                        .inner_radius(inner_fill_radius)
                        .outer_radius(outer_fill_radius);
                    let step_turns = sweep_turns / segments as f32;
                    let overlap = (step_turns * TAU) * 0.35;

                    for i in 0..segments {
                        let t0 = i as f32 / segments as f32;
                        let t1 = (i + 1) as f32 / segments as f32;
                        let t_mid = (t0 + t1) * 0.5;
                        let logical = logical_position(reversed, t_mid);
                        let color = Hsv {
                            h: hue,
                            s: logical,
                            v: hsv_value,
                            a: 1.0,
                        }
                        .to_hsla_ext();

                        let start_angle = (start_turn + t0 * sweep_turns) * TAU - overlap;
                        let end_angle = (start_turn + t1 * sweep_turns) * TAU + overlap;

                        arc_shape.paint(
                            &ArcData {
                                data: &(),
                                index: i,
                                value: 1.0,
                                start_angle,
                                end_angle,
                                pad_angle: 0.0,
                            },
                            color,
                            None,
                            None,
                            &bounds,
                            window,
                        );
                    }

                    paint_arc_end_caps(
                        window,
                        bounds,
                        start_turn,
                        sweep_turns,
                        fill_thickness,
                        Some(outer_fill_radius),
                        Hsv {
                            h: hue,
                            s: logical_position(reversed, 0.0),
                            v: hsv_value,
                            a: 1.0,
                        }
                        .to_hsla_ext(),
                        Hsv {
                            h: hue,
                            s: logical_position(reversed, 1.0),
                            v: hsv_value,
                            a: 1.0,
                        }
                        .to_hsla_ext(),
                    );
                },
            )
            .absolute()
            .inset_0(),
        )
    }

    fn get_color_at_position(&self, arc: &ColorArcState, position: f32) -> Hsla {
        Hsv {
            h: normalize_hue_degrees(self.hue),
            s: logical_position(arc.reversed, position),
            v: self.hsv_value.clamp(0.0, 1.0),
            a: 1.0,
        }
        .to_hsla_ext()
    }
}

pub struct LightnessArcDelegate {
    pub hue: f32,
    pub saturation: f32,
}

impl ColorArcDelegate for LightnessArcDelegate {
    fn style_background(
        &self,
        arc: &ColorArcState,
        container: Div,
        _window: &mut Window,
        _cx: &App,
    ) -> Div {
        let hue = normalize_hue_degrees(self.hue) / 360.0;
        let saturation = self.saturation.clamp(0.0, 1.0);
        let start_turn = arc.start_turns();
        let sweep_turns = arc.sweep_turns();
        let reversed = arc.reversed;
        let border_color = _cx.theme().border;
        if sweep_turns <= f32::EPSILON {
            return container;
        }

        let segments = segment_count(arc.size, sweep_turns);
        let arc_thickness = arc.arc_thickness_px();
        let border_width = 1.6;
        let fill_thickness = (arc_thickness - border_width * 2.0).max(0.0);

        container.child(
            canvas(
                move |bounds, _, _| bounds,
                move |bounds, _prepaint, window, _| {
                    paint_arc_border_underlay(
                        window,
                        bounds,
                        start_turn,
                        sweep_turns,
                        arc_thickness,
                        border_color,
                    );
                    if fill_thickness <= 0.0 {
                        return;
                    }

                    let radius = bounds.size.width.min(bounds.size.height).as_f32() * 0.5;
                    let outer_fill_radius = (radius - border_width).max(0.0);
                    let inner_fill_radius = (outer_fill_radius - fill_thickness).max(0.0);
                    let arc_shape = Arc::new()
                        .inner_radius(inner_fill_radius)
                        .outer_radius(outer_fill_radius);
                    let step_turns = sweep_turns / segments as f32;
                    let overlap = (step_turns * TAU) * 0.35;

                    for i in 0..segments {
                        let t0 = i as f32 / segments as f32;
                        let t1 = (i + 1) as f32 / segments as f32;
                        let t_mid = (t0 + t1) * 0.5;
                        let logical = logical_position(reversed, t_mid);
                        let color = hsla(hue, saturation, logical, 1.0);

                        let start_angle = (start_turn + t0 * sweep_turns) * TAU - overlap;
                        let end_angle = (start_turn + t1 * sweep_turns) * TAU + overlap;

                        arc_shape.paint(
                            &ArcData {
                                data: &(),
                                index: i,
                                value: 1.0,
                                start_angle,
                                end_angle,
                                pad_angle: 0.0,
                            },
                            color,
                            None,
                            None,
                            &bounds,
                            window,
                        );
                    }

                    paint_arc_end_caps(
                        window,
                        bounds,
                        start_turn,
                        sweep_turns,
                        fill_thickness,
                        Some(outer_fill_radius),
                        hsla(hue, saturation, logical_position(reversed, 0.0), 1.0),
                        hsla(hue, saturation, logical_position(reversed, 1.0), 1.0),
                    );
                },
            )
            .absolute()
            .inset_0(),
        )
    }

    fn get_color_at_position(&self, arc: &ColorArcState, position: f32) -> Hsla {
        hsla(
            normalize_hue_degrees(self.hue) / 360.0,
            self.saturation.clamp(0.0, 1.0),
            logical_position(arc.reversed, position),
            1.0,
        )
    }
}
