//! Pure arithmetic for the tiles canvas: magnetic snapping, boundary
//! constraints, resize math, and grid rounding, plus the undo/redo history
//! record for a tile change.
//!
//! This module decides *where* a tile lands. It draws nothing: the tile
//! frame, the drag-bar chrome, and the resize-handle visuals live beside it
//! in [`super`].

use gpui::{Bounds, Pixels, Point, Size, px, size};
use gpui_base::dock::PanelId;

/// A tile smaller than this on either axis cannot be usefully manipulated.
pub(super) const MINIMUM_SIZE: Size<Pixels> = size(px(100.), px(100.));

/// Height of the tile's drag bar. The snapping arithmetic and the painted bar
/// have to agree on it.
pub(super) const DRAG_BAR_HEIGHT: Pixels = px(30.);

/// Size of the resize-handle hit target at a tile's edge.
pub(super) const HANDLE_SIZE: Pixels = px(5.0);

/// A recorded change to one tile's bounds, for undo/redo.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct TileChange {
    panel: PanelId,
    old_bounds: Bounds<Pixels>,
    new_bounds: Bounds<Pixels>,
}

impl TileChange {
    pub(super) fn new(
        panel: PanelId,
        old_bounds: Bounds<Pixels>,
        new_bounds: Bounds<Pixels>,
    ) -> Self {
        Self {
            panel,
            old_bounds,
            new_bounds,
        }
    }

    pub(super) fn panel(&self) -> PanelId {
        self.panel
    }

    pub(super) fn old_bounds(&self) -> Bounds<Pixels> {
        self.old_bounds
    }

    pub(super) fn new_bounds(&self) -> Bounds<Pixels> {
        self.new_bounds
    }
}

/// Which edge (or corner) of a tile a resize drag is manipulating.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ResizeSide {
    Left,
    Right,
    Top,
    Bottom,
    BottomRight,
}

/// In-flight state for a resize drag: which side is moving, where the pointer
/// began the drag, and the tile bounds recorded at the last processed move
/// event.
///
/// Pointer positions arrive in window coordinates while tile bounds live in
/// canvas coordinates, so the start position is kept for the only measure
/// meaningful across the two: how far the pointer has travelled since the
/// drag began.
#[derive(Clone, Copy, Debug)]
pub(super) struct ResizeDrag {
    side: ResizeSide,
    start_position: Point<Pixels>,
    last_bounds: Bounds<Pixels>,
}

impl ResizeDrag {
    pub(super) fn new(
        side: ResizeSide,
        start_position: Point<Pixels>,
        last_bounds: Bounds<Pixels>,
    ) -> Self {
        Self {
            side,
            start_position,
            last_bounds,
        }
    }

    pub(super) fn side(&self) -> ResizeSide {
        self.side
    }

    pub(super) fn start_position(&self) -> Point<Pixels> {
        self.start_position
    }

    pub(super) fn last_bounds(&self) -> Bounds<Pixels> {
        self.last_bounds
    }

    pub(super) fn with_last_bounds(mut self, last_bounds: Bounds<Pixels>) -> Self {
        self.last_bounds = last_bounds;
        self
    }
}

/// Snap `edge` to the nearest value in `candidates` whose distance is strictly
/// below `threshold`. Returns `None` when nothing is close enough.
pub(super) fn snap_edge(edge: Pixels, candidates: &[Pixels], threshold: Pixels) -> Option<Pixels> {
    let mut best: Option<Pixels> = None;
    let mut best_dist = threshold;
    for &candidate in candidates {
        let dist = (edge - candidate).abs();
        if dist < best_dist {
            best_dist = dist;
            best = Some(candidate);
        }
    }
    best
}

/// Compute the final bounds for a resize, applying magnetic edge snapping to
/// neighboring panels and falling back to grid rounding when no neighbor edge
/// is within `grid_size`.
///
/// Which edges move is inferred from the provided `Option`s:
/// - `new_x` set                  => left edge moves (right edge pinned)
/// - `new_width` set, `new_x` not => right edge moves (left edge pinned)
/// - `new_y` set                  => top edge moves (bottom edge pinned)
/// - `new_height` set, `new_y` not => bottom edge moves (top edge pinned)
pub(super) fn compute_resized_bounds(
    previous: Bounds<Pixels>,
    new_x: Option<Pixels>,
    new_y: Option<Pixels>,
    new_width: Option<Pixels>,
    new_height: Option<Pixels>,
    other_bounds: &[Bounds<Pixels>],
    grid_size: Pixels,
) -> Bounds<Pixels> {
    // Candidate snap edges from neighbouring panels.
    let mut x_edges = Vec::with_capacity(other_bounds.len() * 2);
    let mut y_edges = Vec::with_capacity(other_bounds.len() * 2);
    for bounds in other_bounds {
        x_edges.push(bounds.left());
        x_edges.push(bounds.right());
        y_edges.push(bounds.top());
        y_edges.push(bounds.bottom());
    }

    let prev_right = previous.origin.x + previous.size.width;
    let prev_bottom = previous.origin.y + previous.size.height;

    // --- X axis ---
    let (final_x, final_width) = if let Some(x) = new_x {
        // Left edge moving; right edge pinned. Canvas-left (0) is also a target.
        let raw_left = x.max(px(0.));
        let mut candidates = x_edges.clone();
        candidates.push(px(0.));
        let snapped_left = snap_edge(raw_left, &candidates, grid_size)
            .unwrap_or_else(|| round_to_grid(raw_left, grid_size));
        let width = (prev_right - snapped_left).max(MINIMUM_SIZE.width);
        (snapped_left, width)
    } else if let Some(width) = new_width {
        // Right edge moving; left edge pinned.
        let raw_right = previous.origin.x + width;
        let snapped_right = snap_edge(raw_right, &x_edges, grid_size)
            .unwrap_or_else(|| round_to_grid(raw_right, grid_size));
        let width = (snapped_right - previous.origin.x).max(MINIMUM_SIZE.width);
        (previous.origin.x, width)
    } else {
        (previous.origin.x, previous.size.width)
    };

    // --- Y axis ---
    let (final_y, final_height) = if let Some(y) = new_y {
        // Top edge moving; bottom edge pinned. Canvas-top (0) is also a target.
        let raw_top = y.max(px(0.));
        let mut candidates = y_edges.clone();
        candidates.push(px(0.));
        let snapped_top = snap_edge(raw_top, &candidates, grid_size)
            .unwrap_or_else(|| round_to_grid(raw_top, grid_size));
        let height = (prev_bottom - snapped_top).max(MINIMUM_SIZE.height);
        (snapped_top, height)
    } else if let Some(height) = new_height {
        // Bottom edge moving; top edge pinned.
        let raw_bottom = previous.origin.y + height;
        let snapped_bottom = snap_edge(raw_bottom, &y_edges, grid_size)
            .unwrap_or_else(|| round_to_grid(raw_bottom, grid_size));
        let height = (snapped_bottom - previous.origin.y).max(MINIMUM_SIZE.height);
        (previous.origin.y, height)
    } else {
        (previous.origin.y, previous.size.height)
    };

    Bounds {
        origin: Point {
            x: final_x,
            y: final_y,
        },
        size: Size {
            width: final_width,
            height: final_height,
        },
    }
}

/// Round `value` to the nearest multiple of `grid_size`.
pub(super) fn round_to_grid(value: Pixels, grid_size: Pixels) -> Pixels {
    (value / grid_size).round() * grid_size
}

/// Calculate the magnetic snap position for a tile being dragged.
///
/// `moving` is the tile's candidate bounds (already translated by the drag
/// delta, before snapping). `others` are the bounds of every other tile in
/// the same canvas. The returned point keeps `moving.origin`'s coordinate on
/// any axis that did not snap, so the result can be assigned directly as the
/// tile's new origin.
pub(super) fn magnetic_snap(
    moving: Bounds<Pixels>,
    others: &[Bounds<Pixels>],
    threshold: Pixels,
) -> Point<Pixels> {
    // Only check nearby panels
    let search_bounds = Bounds {
        origin: Point {
            x: moving.left() - threshold,
            y: moving.top() - threshold,
        },
        size: Size {
            width: moving.size.width + threshold * 2.0,
            height: moving.size.height + threshold * 2.0,
        },
    };

    let mut snap_x: Option<Pixels> = None;
    let mut snap_y: Option<Pixels> = None;
    let mut min_x_dist = threshold;
    let mut min_y_dist = threshold;

    let drag_left = moving.left();
    let drag_right = moving.right();
    let drag_top = moving.top();
    let drag_bottom = moving.bottom();
    let drag_width = moving.size.width;
    let drag_height = moving.size.height;

    // Check for edge snapping first (top and left boundaries)
    let edge_snap_pos = px(0.);

    // Snap to top edge
    let top_dist = drag_top.abs();
    if top_dist < threshold {
        snap_y = Some(edge_snap_pos);
        min_y_dist = top_dist;
    }

    // Snap to left edge
    let left_dist = drag_left.abs();
    if left_dist < threshold {
        snap_x = Some(edge_snap_pos);
        min_x_dist = left_dist;
    }

    // If both edges are snapped, skip the neighbor search entirely.
    if snap_x.is_none() || snap_y.is_none() {
        for other in others {
            if snap_x.is_some() && snap_y.is_some() {
                break;
            }

            let other_left = other.left();
            let other_right = other.right();
            let other_top = other.top();
            let other_bottom = other.bottom();

            // Skip panels that are far away
            if other_right < search_bounds.left()
                || other_left > search_bounds.right()
                || other_bottom < search_bounds.top()
                || other_top > search_bounds.bottom()
            {
                continue;
            }

            // Horizontal snapping (X axis) - find closest snap point
            if snap_x.is_none() {
                let candidates = [
                    ((drag_left - other_left).abs(), other_left),
                    ((drag_left - other_right).abs(), other_right),
                    ((drag_right - other_left).abs(), other_left - drag_width),
                    ((drag_right - other_right).abs(), other_right - drag_width),
                ];

                for (dist, snap_pos) in candidates {
                    if dist < min_x_dist {
                        min_x_dist = dist;
                        snap_x = Some(snap_pos);
                    }
                }
            }

            // Vertical snapping (Y axis) - find closest snap point
            if snap_y.is_none() {
                let candidates = [
                    ((drag_top - other_top).abs(), other_top),
                    ((drag_top - other_bottom).abs(), other_bottom),
                    ((drag_bottom - other_top).abs(), other_top - drag_height),
                    (
                        (drag_bottom - other_bottom).abs(),
                        other_bottom - drag_height,
                    ),
                ];

                for (dist, snap_pos) in candidates {
                    if dist < min_y_dist {
                        min_y_dist = dist;
                        snap_y = Some(snap_pos);
                    }
                }
            }
        }
    }

    Point {
        x: snap_x.unwrap_or(moving.origin.x),
        y: snap_y.unwrap_or(moving.origin.y),
    }
}

/// Clamp a dragged tile's origin to the canvas boundary.
///
/// The top is a hard boundary (a tile's top can never go negative), and at
/// most `dragging_width - 64px` of the tile may hang off the left edge,
/// keeping 64px of it visible. There is no boundary on the right or bottom:
/// the canvas scrolls.
pub(super) fn apply_boundary_constraints(
    origin: Point<Pixels>,
    dragging_width: Pixels,
) -> Point<Pixels> {
    let mut origin = origin;

    // Top boundary
    if origin.y < px(0.) {
        origin.y = px(0.);
    }

    // Left boundary (allow partial off-screen but keep 64px visible)
    let min_left = -dragging_width + px(64.);
    if origin.x < min_left {
        origin.x = min_left;
    }

    origin
}

/// The scrollable extent a set of tiles occupies, measured from the canvas
/// origin.
///
/// The union runs from `min(0, left)` to `max(0, right)` on each axis, so a
/// canvas whose tiles all sit at positive coordinates reports the far edge,
/// and one with a tile dragged past the origin reports the distance across
/// both.
pub(super) fn content_size(tiles: &[Bounds<Pixels>]) -> Size<Pixels> {
    let mut left = px(0.);
    let mut top = px(0.);
    let mut right = px(0.);
    let mut bottom = px(0.);
    for bounds in tiles {
        left = left.min(bounds.left());
        top = top.min(bounds.top());
        right = right.max(bounds.right());
        bottom = bottom.max(bounds.bottom());
    }
    size(right - left, bottom - top)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds {
            origin: Point { x: px(x), y: px(y) },
            size: Size {
                width: px(w),
                height: px(h),
            },
        }
    }

    #[test]
    fn test_snap_edge_within_threshold() {
        // 102 is 2px from 100 (< 8) -> snaps to 100.
        assert_eq!(
            snap_edge(px(102.), &[px(100.), px(300.)], px(8.)),
            Some(px(100.))
        );
    }

    #[test]
    fn test_snap_edge_outside_threshold() {
        // 120 is 20px from nearest candidate (>= 8) -> no snap.
        assert_eq!(snap_edge(px(120.), &[px(100.), px(300.)], px(8.)), None);
    }

    #[test]
    fn test_snap_edge_picks_nearest() {
        // 303 is 3px from 300 and 5px from 308 -> picks 300.
        assert_eq!(
            snap_edge(px(303.), &[px(308.), px(300.)], px(8.)),
            Some(px(300.))
        );
    }

    #[test]
    fn test_snap_edge_empty_candidates() {
        assert_eq!(snap_edge(px(50.), &[], px(8.)), None);
    }

    #[test]
    fn test_resize_right_edge_snaps_to_neighbor_left() {
        // Panel A: x=0 w=196 (right edge 196). Neighbour B starts at x=200.
        // Dragging right edge to 197 should snap right edge to 200 -> width 200.
        let prev = b(0., 0., 196., 100.);
        let neighbor = b(200., 0., 100., 100.);
        let out =
            compute_resized_bounds(prev, None, None, Some(px(197.)), None, &[neighbor], px(8.));
        assert_eq!(out.origin.x, px(0.));
        assert_eq!(out.size.width, px(200.));
    }

    #[test]
    fn test_resize_bottom_edge_snaps_to_neighbor_top() {
        let prev = b(0., 0., 100., 196.);
        let neighbor = b(0., 200., 100., 100.);
        let out =
            compute_resized_bounds(prev, None, None, None, Some(px(197.)), &[neighbor], px(8.));
        assert_eq!(out.origin.y, px(0.));
        assert_eq!(out.size.height, px(200.));
    }

    #[test]
    fn test_resize_left_edge_snaps_and_pins_right() {
        // Panel: x=200 w=100 (right edge 300). Neighbour right edge at 100.
        // Drag left edge to 103 -> snaps to 100 -> width = 300 - 100 = 200.
        let prev = b(200., 0., 100., 100.);
        let neighbor = b(0., 0., 100., 100.);
        let out = compute_resized_bounds(
            prev,
            Some(px(103.)),
            None,
            Some(px(197.)),
            None,
            &[neighbor],
            px(8.),
        );
        assert_eq!(out.origin.x, px(100.));
        assert_eq!(out.size.width, px(200.));
    }

    #[test]
    fn test_resize_corner_snaps_both_edges() {
        // Right edge -> neighbour-right at 300; bottom edge -> neighbour-bottom at 250.
        let prev = b(0., 0., 196., 196.);
        let right_neighbor = b(100., 0., 200., 100.); // right edge = 300
        let bottom_neighbor = b(0., 100., 100., 150.); // bottom edge = 250
        let out = compute_resized_bounds(
            prev,
            None,
            None,
            Some(px(298.)),
            Some(px(248.)),
            &[right_neighbor, bottom_neighbor],
            px(8.),
        );
        assert_eq!(out.size.width, px(300.));
        assert_eq!(out.size.height, px(250.));
    }

    #[test]
    fn test_resize_grid_rounds_when_no_neighbor_close() {
        // No neighbours; raw right edge 153 -> grid round to 152 (nearest multiple of 8).
        let prev = b(0., 0., 100., 100.);
        let out = compute_resized_bounds(prev, None, None, Some(px(153.)), None, &[], px(8.));
        assert_eq!(out.size.width, px(152.));
    }

    #[test]
    fn test_resize_respects_minimum_size() {
        let prev = b(0., 0., 100., 100.);
        let out = compute_resized_bounds(prev, None, None, Some(px(10.)), None, &[], px(8.));
        assert_eq!(out.size.width, MINIMUM_SIZE.width);
    }

    #[test]
    fn content_size_spans_from_the_origin_to_the_far_edge() {
        assert_eq!(
            content_size(&[b(20., 20., 380., 280.), b(420., 20., 380., 280.)]),
            size(px(800.), px(300.)),
            "the extent runs from the canvas origin, not from the first tile"
        );
        assert_eq!(
            content_size(&[]),
            size(px(0.), px(0.)),
            "an empty canvas scrolls nowhere"
        );
        assert_eq!(
            content_size(&[b(-40., -10., 100., 100.)]),
            size(px(100.), px(100.)),
            "a tile dragged past the origin still reports the distance across it"
        );
    }

    #[test]
    fn test_resize_no_change_returns_previous_geometry() {
        let prev = b(0., 0., 100., 100.);
        let out = compute_resized_bounds(prev, None, None, None, None, &[], px(8.));
        assert_eq!(out.origin.x, px(0.));
        assert_eq!(out.origin.y, px(0.));
        assert_eq!(out.size.width, px(100.));
        assert_eq!(out.size.height, px(100.));
    }
}
