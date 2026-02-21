use gpui::{px, Bounds, Modifiers, Pixels, Point};
use gpui_component::PixelsExt as _;
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub(crate) struct RingGeometry {
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

pub(crate) fn mirrored_saturation(position: f32) -> f32 {
    (1.0 - (position.rem_euclid(1.0) - 0.5).abs() * 2.0).clamp(0.0, 1.0)
}

pub(crate) fn mirrored_lightness(position: f32) -> f32 {
    ((position.rem_euclid(1.0) - 0.5).abs() * 2.0).clamp(0.0, 1.0)
}

pub(crate) fn position_from_mirrored_saturation(value: f32) -> f32 {
    (1.0 - value.clamp(0.0, 1.0) * 0.5).rem_euclid(1.0)
}

pub(crate) fn position_from_mirrored_lightness(value: f32) -> f32 {
    (0.5 + value.clamp(0.0, 1.0) * 0.5).rem_euclid(1.0)
}

pub(crate) fn ring_geometry(bounds: Bounds<Pixels>, ring_thickness: f32) -> Option<RingGeometry> {
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
    let inner_radius = (outer_radius - ring_thickness).max(0.0);
    let track_radius = (outer_radius - ring_thickness / 2.0).max(0.0);

    Some(RingGeometry {
        center_x,
        center_y,
        outer_radius,
        inner_radius,
        track_radius,
    })
}

pub(crate) fn pointer_to_ring_position(
    bounds: Bounds<Pixels>,
    pointer: Point<Pixels>,
    rotation_turns: f32,
    reversed: bool,
) -> Option<f32> {
    let geometry = ring_geometry(bounds, 0.0)?;
    let pointer_x: f32 = pointer.x.into();
    let pointer_y: f32 = pointer.y.into();
    let dx = pointer_x - geometry.center_x;
    let dy = pointer_y - geometry.center_y;
    let theta = dy.atan2(dx);
    let unit = (theta / TAU).rem_euclid(1.0);
    let mut position = (0.25 + unit - rotation_turns).rem_euclid(1.0);
    if reversed {
        position = 1.0 - position;
    }
    Some(position)
}

pub(crate) fn position_to_theta(position: f32, rotation_turns: f32) -> f32 {
    (position - 0.25 + rotation_turns) * TAU
}

pub(crate) fn thumb_center(geometry: RingGeometry, theta: f32) -> (f32, f32) {
    (
        geometry.center_x + geometry.track_radius * theta.cos(),
        geometry.center_y + geometry.track_radius * theta.sin(),
    )
}

pub(crate) fn thumb_top_left(geometry: RingGeometry, theta: f32, thumb_size: f32) -> (f32, f32) {
    let (center_x, center_y) = thumb_center(geometry, theta);
    let half = thumb_size / 2.0;
    (center_x - half, center_y - half)
}

pub(crate) fn pointer_in_thumb_box(
    pointer: Point<Pixels>,
    geometry: RingGeometry,
    theta: f32,
    thumb_size: f32,
) -> bool {
    let (thumb_center_x, thumb_center_y) = thumb_center(geometry, theta);
    let half = thumb_size / 2.0;
    let pointer_x: f32 = pointer.x.into();
    let pointer_y: f32 = pointer.y.into();
    pointer_x >= thumb_center_x - half
        && pointer_x <= thumb_center_x + half
        && pointer_y >= thumb_center_y - half
        && pointer_y <= thumb_center_y + half
}

pub(crate) fn pointer_hits_active_target(
    pointer: Point<Pixels>,
    geometry: RingGeometry,
    theta: f32,
    thumb_size: f32,
    allow_inner_target: bool,
) -> bool {
    let pointer_x: f32 = pointer.x.into();
    let pointer_y: f32 = pointer.y.into();
    let dx = pointer_x - geometry.center_x;
    let dy = pointer_y - geometry.center_y;
    let radius = (dx * dx + dy * dy).sqrt();
    let on_ring = radius >= geometry.inner_radius && radius <= geometry.outer_radius;
    let on_active_disk = allow_inner_target && radius <= geometry.outer_radius;
    let on_thumb = pointer_in_thumb_box(pointer, geometry, theta, thumb_size);
    on_ring || on_active_disk || on_thumb
}

pub(crate) fn start_drag(drag_interaction_enabled: &mut bool, accepts_pointer: bool) -> bool {
    *drag_interaction_enabled = accepts_pointer;
    accepts_pointer
}

pub(crate) fn end_drag(drag_interaction_enabled: &mut bool) -> bool {
    if !*drag_interaction_enabled {
        return false;
    }
    *drag_interaction_enabled = false;
    true
}

pub(crate) fn next_position_for_arrow_key(
    key: &str,
    modifiers: Modifiers,
    range: std::ops::Range<f32>,
    step: Option<f32>,
    reversed: bool,
    current_position: f32,
) -> Option<f32> {
    let mut delta_sign = match key {
        "left" | "up" => -1.0,
        "right" | "down" => 1.0,
        _ => return None,
    };

    let range_span = range.end - range.start;
    if range_span.abs() <= f32::EPSILON {
        return None;
    }

    let default_step = if range_span.abs() > 10.0 {
        1.0
    } else {
        (range_span.abs() / 100.0).max(0.0001)
    };
    let base_step = step.unwrap_or(default_step).abs();
    let multiplier = if modifiers.shift {
        10.0
    } else if modifiers.alt {
        0.1
    } else {
        1.0
    };
    let step_size = base_step * multiplier;
    let step_position = (step_size / range_span.abs()).clamp(0.0, 1.0);
    if step_position <= 0.0 {
        return None;
    }

    if reversed {
        delta_sign = -delta_sign;
    }
    Some((current_position + delta_sign * step_position).rem_euclid(1.0))
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
    fn value_percent_handles_regular_and_degenerate_ranges() {
        approx_eq(value_percent(25.0, 0.0..100.0), 0.25);
        approx_eq(value_percent(150.0, 0.0..100.0), 1.0);
        approx_eq(value_percent(-10.0, 0.0..100.0), 0.0);
        approx_eq(value_percent(10.0, 5.0..5.0), 0.0);
    }

    #[test]
    fn mirrored_mappings_round_trip_via_inverse_position_functions() {
        for value in [0.0, 0.2, 0.5, 0.8, 1.0] {
            let sat_pos = position_from_mirrored_saturation(value);
            approx_eq(mirrored_saturation(sat_pos), value);

            let light_pos = position_from_mirrored_lightness(value);
            approx_eq(mirrored_lightness(light_pos), value);
        }
    }

    #[test]
    fn effective_position_prefers_active_interaction_when_values_match() {
        let resolved = effective_position(0.6, Some(0.42), |v| v, |p| p + 0.18);
        approx_eq(resolved, 0.42);

        let fallback = effective_position(0.6, Some(0.42), |v| v, |p| p);
        approx_eq(fallback, 0.6);
    }

    #[test]
    fn pointer_to_ring_position_maps_cardinal_directions() {
        let bounds = unit_bounds();

        let top = pointer_to_ring_position(bounds, point(px(50.0), px(0.0)), 0.0, false).unwrap();
        let right =
            pointer_to_ring_position(bounds, point(px(100.0), px(50.0)), 0.0, false).unwrap();
        let bottom =
            pointer_to_ring_position(bounds, point(px(50.0), px(100.0)), 0.0, false).unwrap();
        let left = pointer_to_ring_position(bounds, point(px(0.0), px(50.0)), 0.0, false).unwrap();

        approx_eq(top, 0.0);
        approx_eq(right, 0.25);
        approx_eq(bottom, 0.5);
        approx_eq(left, 0.75);

        let reversed_right =
            pointer_to_ring_position(bounds, point(px(100.0), px(50.0)), 0.0, true).unwrap();
        approx_eq(reversed_right, 0.75);
    }

    #[test]
    fn theta_and_thumb_geometry_helpers_are_consistent() {
        let geometry = RingGeometry {
            center_x: 10.0,
            center_y: 20.0,
            outer_radius: 30.0,
            inner_radius: 20.0,
            track_radius: 25.0,
        };

        let theta = position_to_theta(0.25, 0.0);
        approx_eq(theta, 0.0);

        let (cx, cy) = thumb_center(geometry, theta);
        approx_eq(cx, 35.0);
        approx_eq(cy, 20.0);

        let (tx, ty) = thumb_top_left(geometry, theta, 10.0);
        approx_eq(tx, 30.0);
        approx_eq(ty, 15.0);
    }

    #[test]
    fn pointer_hits_active_target_checks_ring_inner_and_thumb_areas() {
        let geometry = RingGeometry {
            center_x: 50.0,
            center_y: 50.0,
            outer_radius: 40.0,
            inner_radius: 30.0,
            track_radius: 35.0,
        };
        let theta = 0.0;
        let thumb_size = 12.0;

        let ring_point = point(px(85.0), px(50.0));
        assert!(pointer_hits_active_target(
            ring_point, geometry, theta, thumb_size, false
        ));

        let center_point = point(px(50.0), px(50.0));
        assert!(!pointer_hits_active_target(
            center_point,
            geometry,
            theta,
            thumb_size,
            false
        ));
        assert!(pointer_hits_active_target(
            center_point,
            geometry,
            theta,
            thumb_size,
            true
        ));
    }

    #[test]
    fn next_position_for_arrow_key_respects_modifiers_and_reversed() {
        let none = Modifiers::default();
        let shift = Modifiers {
            shift: true,
            ..Default::default()
        };
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };

        let base =
            next_position_for_arrow_key("right", none, 0.0..1.0, Some(0.1), false, 0.0).unwrap();
        approx_eq(base, 0.1);

        let shifted =
            next_position_for_arrow_key("right", shift, 0.0..1.0, Some(0.1), false, 0.0).unwrap();
        approx_eq(shifted, 0.0);

        let alted =
            next_position_for_arrow_key("right", alt, 0.0..1.0, Some(0.1), false, 0.0).unwrap();
        approx_eq(alted, 0.01);

        let reversed =
            next_position_for_arrow_key("right", none, 0.0..1.0, Some(0.1), true, 0.0).unwrap();
        approx_eq(reversed, 0.9);

        assert!(
            next_position_for_arrow_key("space", none, 0.0..1.0, Some(0.1), false, 0.0).is_none()
        );
        assert!(
            next_position_for_arrow_key("right", none, 2.0..2.0, Some(0.1), false, 0.0).is_none()
        );
    }

    #[test]
    fn end_drag_disables_drag_only_when_active() {
        let mut dragging = true;

        assert!(end_drag(&mut dragging));
        assert!(!dragging);

        assert!(!end_drag(&mut dragging));
        assert!(!dragging);
    }

    #[test]
    fn value_from_position_snaps_relative_to_range_start() {
        let identity = |p: f32| p;

        approx_eq(
            value_from_position(0.24, 3.0..13.0, Some(2.0), identity),
            5.0,
        );
        approx_eq(
            value_from_position(0.24, 13.0..3.0, Some(2.0), identity),
            11.0,
        );
        approx_eq(
            value_from_position(0.24, 3.0..13.0, Some(0.0), identity),
            5.4,
        );
    }
}
