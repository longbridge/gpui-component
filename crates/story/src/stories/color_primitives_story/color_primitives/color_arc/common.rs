#![allow(dead_code)]

use gpui::{px, Bounds, Pixels, Point};
use gpui_component::PixelsExt as _;
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub(crate) struct ArcGeometry {
    pub center_x: f32,
    pub center_y: f32,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub track_radius: f32,
}

pub(crate) fn size_px(size: gpui_component::Size) -> f32 {
    match size {
        gpui_component::Size::XSmall => 140.0,
        gpui_component::Size::Small => 180.0,
        gpui_component::Size::Medium => 220.0,
        gpui_component::Size::Large => 280.0,
        gpui_component::Size::Size(px) => px.as_f32(),
    }
}

pub(crate) fn normalize_turn(turn: f32) -> f32 {
    turn.rem_euclid(1.0)
}

pub(crate) fn start_turns(start_degrees: f32) -> f32 {
    normalize_turn(start_degrees / 360.0)
}

pub(crate) fn sweep_turns(sweep_degrees: f32) -> f32 {
    (sweep_degrees / 360.0).clamp(0.0, 1.0)
}

pub(crate) fn value_percent(value: f32, range: std::ops::Range<f32>) -> f32 {
    let range_span = range.end - range.start;
    if range_span.abs() <= f32::EPSILON {
        return 0.0;
    }

    ((value - range.start) / range_span).clamp(0.0, 1.0)
}

pub(crate) fn effective_position(
    value_percent: f32,
    interaction_position: Option<f32>,
    value_to_position: impl Fn(f32) -> f32,
    position_to_value: impl Fn(f32) -> f32,
) -> f32 {
    let mut resolved = value_to_position(value_percent);
    if let Some(position) = interaction_position {
        let mapped = position_to_value(position);
        if (mapped - value_percent).abs() <= 0.001 {
            resolved = position;
        }
    }
    resolved
}

pub(crate) fn value_from_position(
    position: f32,
    range: std::ops::Range<f32>,
    step: Option<f32>,
    position_to_value_percent: impl Fn(f32) -> f32,
) -> f32 {
    let range_span = range.end - range.start;
    let pct = position_to_value_percent(position);
    let mut value = range.start + range_span * pct;
    if let Some(step) = step {
        let step = step.abs();
        if step > f32::EPSILON {
            value = range.start + ((value - range.start) / step).round() * step;
        }
    }
    value.clamp(range.start.min(range.end), range.end.max(range.start))
}

pub(crate) fn arc_geometry(bounds: Bounds<Pixels>, arc_thickness: f32) -> Option<ArcGeometry> {
    if bounds.size.width <= px(0.0) || bounds.size.height <= px(0.0) {
        return None;
    }

    let width: f32 = bounds.size.width.into();
    let height: f32 = bounds.size.height.into();
    let origin_x: f32 = bounds.origin.x.into();
    let origin_y: f32 = bounds.origin.y.into();
    let center_x = origin_x + width / 2.0;
    let center_y = origin_y + height / 2.0;
    let outer_radius = width.min(height) / 2.0;
    let inner_radius = (outer_radius - arc_thickness).max(0.0);
    let track_radius = (outer_radius - arc_thickness / 2.0).max(0.0);

    Some(ArcGeometry {
        center_x,
        center_y,
        outer_radius,
        inner_radius,
        track_radius,
    })
}

pub(crate) fn pointer_to_turn(bounds: Bounds<Pixels>, pointer: Point<Pixels>) -> Option<f32> {
    let geometry = arc_geometry(bounds, 0.0)?;
    Some(pointer_to_turn_from_center(
        pointer,
        geometry.center_x,
        geometry.center_y,
    ))
}

fn pointer_to_turn_from_center(pointer: Point<Pixels>, center_x: f32, center_y: f32) -> f32 {
    let pointer_x: f32 = pointer.x.into();
    let pointer_y: f32 = pointer.y.into();
    let dx = pointer_x - center_x;
    let dy = pointer_y - center_y;
    let theta = dy.atan2(dx);
    let unit = (theta / TAU).rem_euclid(1.0);
    (0.25 + unit).rem_euclid(1.0)
}

pub(crate) fn turn_to_theta(turn: f32) -> f32 {
    (normalize_turn(turn) - 0.25) * TAU
}

fn angular_distance_turn(a: f32, b: f32) -> f32 {
    let delta = (normalize_turn(a) - normalize_turn(b)).abs();
    delta.min(1.0 - delta)
}

pub(crate) fn arc_contains_turn(turn: f32, arc_start_turn: f32, arc_sweep_turn: f32) -> bool {
    if arc_sweep_turn >= 1.0 {
        return true;
    }
    if arc_sweep_turn <= f32::EPSILON {
        return false;
    }
    let delta = (normalize_turn(turn) - normalize_turn(arc_start_turn)).rem_euclid(1.0);
    delta <= arc_sweep_turn
}

pub(crate) fn project_turn_to_arc(turn: f32, arc_start_turn: f32, arc_sweep_turn: f32) -> f32 {
    if arc_contains_turn(turn, arc_start_turn, arc_sweep_turn) {
        return normalize_turn(turn);
    }

    if arc_sweep_turn <= f32::EPSILON {
        return normalize_turn(arc_start_turn);
    }

    let start = normalize_turn(arc_start_turn);
    let end = normalize_turn(arc_start_turn + arc_sweep_turn);

    let to_start = angular_distance_turn(turn, start);
    let to_end = angular_distance_turn(turn, end);
    if to_start <= to_end {
        start
    } else {
        end
    }
}

pub(crate) fn turn_to_position(turn: f32, arc_start_turn: f32, arc_sweep_turn: f32) -> f32 {
    if arc_sweep_turn <= f32::EPSILON {
        return 0.0;
    }
    ((normalize_turn(turn) - normalize_turn(arc_start_turn)).rem_euclid(1.0) / arc_sweep_turn)
        .clamp(0.0, 1.0)
}

pub(crate) fn position_to_turn(position: f32, arc_start_turn: f32, arc_sweep_turn: f32) -> f32 {
    (normalize_turn(arc_start_turn) + position.clamp(0.0, 1.0) * arc_sweep_turn).rem_euclid(1.0)
}

pub(crate) fn pointer_to_arc_position(
    bounds: Bounds<Pixels>,
    pointer: Point<Pixels>,
    arc_start_turn: f32,
    arc_sweep_turn: f32,
    reversed: bool,
) -> Option<f32> {
    if arc_sweep_turn <= f32::EPSILON {
        return None;
    }

    let turn = pointer_to_turn(bounds, pointer)?;
    let clamped_turn = project_turn_to_arc(turn, arc_start_turn, arc_sweep_turn);
    let mut position = turn_to_position(clamped_turn, arc_start_turn, arc_sweep_turn);
    if reversed {
        position = 1.0 - position;
    }
    Some(position)
}

pub(crate) fn thumb_top_left(geometry: ArcGeometry, turn: f32, thumb_size: f32) -> (f32, f32) {
    let theta = turn_to_theta(turn);
    let center_x = geometry.center_x + geometry.track_radius * theta.cos();
    let center_y = geometry.center_y + geometry.track_radius * theta.sin();
    let half = thumb_size / 2.0;
    (center_x - half, center_y - half)
}

pub(crate) fn pointer_in_thumb_box(
    pointer: Point<Pixels>,
    geometry: ArcGeometry,
    turn: f32,
    thumb_size: f32,
) -> bool {
    let theta = turn_to_theta(turn);
    let thumb_center_x = geometry.center_x + geometry.track_radius * theta.cos();
    let thumb_center_y = geometry.center_y + geometry.track_radius * theta.sin();
    let half = thumb_size / 2.0;
    let pointer_x: f32 = pointer.x.into();
    let pointer_y: f32 = pointer.y.into();
    pointer_x >= thumb_center_x - half
        && pointer_x <= thumb_center_x + half
        && pointer_y >= thumb_center_y - half
        && pointer_y <= thumb_center_y + half
}

pub(crate) fn pointer_hits_arc_target(
    pointer: Point<Pixels>,
    geometry: ArcGeometry,
    arc_start_turn: f32,
    arc_sweep_turn: f32,
    thumb_turn: f32,
    thumb_size: f32,
) -> bool {
    let pointer_x: f32 = pointer.x.into();
    let pointer_y: f32 = pointer.y.into();
    let dx = pointer_x - geometry.center_x;
    let dy = pointer_y - geometry.center_y;
    let radius = (dx * dx + dy * dy).sqrt();

    let on_ring = radius >= geometry.inner_radius && radius <= geometry.outer_radius;
    let turn = pointer_to_turn_from_center(pointer, geometry.center_x, geometry.center_y);
    let on_arc = on_ring && arc_contains_turn(turn, arc_start_turn, arc_sweep_turn);

    on_arc || pointer_in_thumb_box(pointer, geometry, thumb_turn, thumb_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};

    fn approx_eq(a: f32, b: f32) {
        assert!(
            (a - b).abs() < 1e-5,
            "expected {a} ~= {b}, delta={}",
            (a - b).abs()
        );
    }

    fn unit_bounds() -> Bounds<Pixels> {
        Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(100.0), px(100.0)),
        }
    }

    #[test]
    fn pointer_to_turn_maps_cardinal_directions() {
        let bounds = unit_bounds();
        let top = pointer_to_turn(bounds, point(px(50.0), px(0.0))).unwrap();
        let right = pointer_to_turn(bounds, point(px(100.0), px(50.0))).unwrap();
        let bottom = pointer_to_turn(bounds, point(px(50.0), px(100.0))).unwrap();
        let left = pointer_to_turn(bounds, point(px(0.0), px(50.0))).unwrap();

        approx_eq(top, 0.0);
        approx_eq(right, 0.25);
        approx_eq(bottom, 0.5);
        approx_eq(left, 0.75);
    }

    #[test]
    fn project_turn_to_arc_clamps_to_nearest_endpoint() {
        let start = 0.0;
        let sweep = 0.5;

        approx_eq(project_turn_to_arc(0.25, start, sweep), 0.25);
        approx_eq(project_turn_to_arc(0.75, start, sweep), 0.0);
        approx_eq(project_turn_to_arc(0.9, start, sweep), 0.0);
    }

    #[test]
    fn pointer_to_arc_position_respects_reversed() {
        let bounds = unit_bounds();
        let start = 0.0;
        let sweep = 0.5;

        let pos = pointer_to_arc_position(bounds, point(px(100.0), px(50.0)), start, sweep, false)
            .unwrap();
        approx_eq(pos, 0.5);

        let reversed =
            pointer_to_arc_position(bounds, point(px(100.0), px(50.0)), start, sweep, true)
                .unwrap();
        approx_eq(reversed, 0.5);
    }
}
