use std::time::Duration;

use gpui::{
    Along, Axis, Bounds, Context, EventEmitter, Pixels, Point, ScrollHandle, TouchPhase, px,
};

const POINTER_AXIS_LOCK_THRESHOLD: Pixels = px(2.);
// Keep this aligned with GPUI's OngoingScroll timeout. Some platforms only
// emit `Moved`, so a quiet period is the only signal that a new gesture began.
const SCROLL_EVENT_SEPARATION: Duration = Duration::from_millis(28);

/// An event emitted when user interaction selects another carousel item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CarouselEvent {
    /// The newly selected item index.
    Change(usize),
}

/// Bounds collected by [`super::CarouselContent`] after layout.
///
/// The bounds are kept in the content's unscrolled coordinate space.  Keeping
/// this geometry in the behavior state lets pointer, wheel, and keyboard
/// input all resolve to the same snap points without making the content own a
/// second copy of the selection state.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct CarouselGeometry {
    viewport: Option<Bounds<Pixels>>,
    items: Vec<Bounds<Pixels>>,
}

#[derive(Clone, Copy, Debug)]
struct PointerGesture {
    start_position: Point<Pixels>,
    start_offset: Point<Pixels>,
    start_index: Option<usize>,
    total_delta: Pixels,
    axis_locked: bool,
}

#[derive(Clone, Copy, Debug)]
struct ScrollGesture {
    start_offset: Point<Pixels>,
    start_index: Option<usize>,
    total_delta: Pixels,
}

/// Shared behavior state for every part of a [`super::Carousel`].
///
/// `CarouselState` intentionally owns behavior only.  The content and its
/// items provide presentation and layout, while this state owns selection,
/// orientation, looping, the shared scroll handle, and the input snapshots
/// used to settle gestures.  Programmatic setters are silent; user-facing
/// selection methods emit one [`CarouselEvent::Change`] for a successful
/// selection.
pub struct CarouselState {
    item_count: usize,
    selected_index: Option<usize>,
    axis: Axis,
    looping: bool,
    scroll_handle: ScrollHandle,
    geometry: CarouselGeometry,
    pointer_gesture: Option<PointerGesture>,
    scroll_gesture: Option<ScrollGesture>,
    ignore_scroll_until_quiet: bool,
    scroll_settle_epoch: usize,
    suppress_pointer_click: bool,
    motion_revision: usize,
}

impl CarouselState {
    /// Creates state for `item_count` items, initially selecting the first
    /// item when at least one item exists.
    pub fn new(item_count: usize) -> Self {
        Self {
            item_count,
            selected_index: (item_count > 0).then_some(0),
            axis: Axis::Horizontal,
            looping: false,
            scroll_handle: ScrollHandle::new(),
            geometry: CarouselGeometry::default(),
            pointer_gesture: None,
            scroll_gesture: None,
            ignore_scroll_until_quiet: false,
            scroll_settle_epoch: 0,
            suppress_pointer_click: false,
            motion_revision: 0,
        }
    }

    /// Sets the initially selected item.
    pub fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = self.clamp_index(index);
        self
    }

    /// Sets the carousel orientation.
    pub fn with_axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Enables or disables wrapping at the first and last items.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Returns the number of logical items.
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Returns the selected logical item, or `None` when the carousel is
    /// empty.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Returns the configured orientation.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// Returns whether navigation wraps at the carousel edges.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Returns whether a previous item can be selected.
    pub fn has_previous(&self) -> bool {
        match self.selected_index {
            Some(_) if self.looping => self.item_count > 1,
            Some(index) => index > 0,
            None => false,
        }
    }

    /// Returns whether a next item can be selected.
    pub fn has_next(&self) -> bool {
        match self.selected_index {
            Some(_) if self.looping => self.item_count > 1,
            Some(index) => index + 1 < self.item_count,
            None => false,
        }
    }

    /// Silently changes the selected item for controlled/programmatic use.
    ///
    /// The value is clamped to the available item range.  This method does
    /// not emit [`CarouselEvent::Change`].
    pub fn set_selected_index(&mut self, index: usize, cx: &mut Context<Self>) {
        let next_index = self.clamp_index(index);
        let changed = self.selected_index != next_index || self.is_interacting();
        if self
            .selected_index
            .zip(next_index)
            .is_some_and(|(current, next)| self.is_loop_wrap(current, next))
        {
            self.motion_revision = self.motion_revision.wrapping_add(1);
        }
        self.selected_index = next_index;
        self.cancel_interactions(cx);
        if changed {
            cx.notify();
        }
    }

    /// Silently changes the number of logical items and clamps the selection.
    pub fn set_item_count(&mut self, item_count: usize, cx: &mut Context<Self>) {
        if self.item_count == item_count {
            return;
        }
        self.item_count = item_count;
        self.selected_index = self
            .selected_index
            .and_then(|index| (item_count > 0).then_some(index.min(item_count.saturating_sub(1))));
        if self.selected_index.is_none() && item_count > 0 {
            self.selected_index = Some(0);
        }
        self.geometry.items.clear();
        self.cancel_interactions(cx);
        cx.notify();
    }

    /// Silently changes the carousel orientation.
    pub fn set_axis(&mut self, axis: Axis, cx: &mut Context<Self>) {
        if self.axis != axis {
            self.axis = axis;
            self.scroll_handle.set_offset(Point::default());
            self.cancel_interactions(cx);
            self.motion_revision = self.motion_revision.wrapping_add(1);
            cx.notify();
        }
    }

    /// Silently changes whether edge navigation wraps.
    pub fn set_looping(&mut self, looping: bool, cx: &mut Context<Self>) {
        if self.looping != looping {
            self.looping = looping;
            self.cancel_interactions(cx);
            cx.notify();
        }
    }

    /// Selects an item through a user-facing path and emits one change event
    /// when the index is valid and differs from the current selection.
    pub fn select_index(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.item_count {
            self.cancel_user_interaction(cx);
            return false;
        }
        let wrapped = self
            .selected_index
            .zip(Some(index))
            .is_some_and(|(current, next)| self.is_loop_wrap(current, next));
        self.select_index_with_wrap(index, wrapped, cx)
    }

    /// Selects the previous item, wrapping when looping is enabled.
    pub fn select_previous(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(current) = self.selected_index else {
            return false;
        };

        if current > 0 {
            self.select_index_with_wrap(current - 1, false, cx)
        } else if self.looping && self.item_count > 1 {
            self.select_index_with_wrap(self.item_count - 1, true, cx)
        } else {
            self.cancel_user_interaction(cx);
            false
        }
    }

    /// Selects the next item, wrapping when looping is enabled.
    pub fn select_next(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(current) = self.selected_index else {
            return false;
        };

        if current + 1 < self.item_count {
            self.select_index_with_wrap(current + 1, false, cx)
        } else if self.looping && self.item_count > 1 {
            self.select_index_with_wrap(0, true, cx)
        } else {
            self.cancel_user_interaction(cx);
            false
        }
    }

    /// Selects the first item through the user-facing path.
    pub fn select_first(&mut self, cx: &mut Context<Self>) -> bool {
        self.select_index(0, cx)
    }

    /// Selects the last item through the user-facing path.
    pub fn select_last(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.item_count.checked_sub(1) else {
            self.cancel_user_interaction(cx);
            return false;
        };
        self.select_index(index, cx)
    }

    /// Returns the shared scroll handle used by the content viewport.
    pub(super) fn scroll_handle(&self) -> &ScrollHandle {
        &self.scroll_handle
    }

    /// Returns whether pointer or trackpad input is currently active.
    pub(super) fn is_interacting(&self) -> bool {
        self.pointer_gesture.is_some() || self.scroll_gesture.is_some()
    }

    /// Returns whether the active pointer gesture has committed to this axis.
    pub(super) fn is_pointer_drag_locked(&self) -> bool {
        self.pointer_gesture
            .is_some_and(|gesture| gesture.axis_locked)
    }

    /// Returns whether a cross-axis move cancelled this pointer sequence.
    ///
    /// The event surface uses this to stop the release during its bubble phase.
    /// Capture-phase handlers still run first, so descendant controls clear
    /// their pending press and ancestor gesture surfaces can finish.
    pub(super) fn should_suppress_pointer_click(&self) -> bool {
        self.suppress_pointer_click
    }

    /// Returns a monotonic key for motion that must be rebased immediately.
    ///
    /// Ordinary adjacent selection leaves this value unchanged.  A logical
    /// loop wrap increments it so the content can use a fresh spring key and
    /// avoid animating across the entire strip.
    pub(super) fn motion_revision(&self) -> usize {
        self.motion_revision
    }

    /// Records the viewport and item bounds used for gesture snapping.
    pub(super) fn set_geometry(&mut self, viewport: Bounds<Pixels>, items: Vec<Bounds<Pixels>>) {
        self.geometry = CarouselGeometry {
            viewport: Some(viewport),
            items,
        };
    }

    /// Returns the geometry-derived snap offset for `index`.
    pub(super) fn snap_target_for(&self, index: usize) -> Option<Point<Pixels>> {
        let viewport = self.geometry.viewport?;
        let item = self.geometry.items.get(index)?;
        Some(self.snap_offset(viewport, *item))
    }

    /// Returns the nearest item index for a scroll offset.
    pub(super) fn nearest_index(&self, offset: Point<Pixels>) -> Option<usize> {
        let viewport = self.geometry.viewport?;
        self.geometry
            .items
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                let left_distance = self.primary_distance(offset, viewport, **left);
                let right_distance = self.primary_distance(offset, viewport, **right);
                left_distance
                    .partial_cmp(&right_distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
    }

    /// Begins a pointer drag and notifies the owning entity.
    pub(super) fn begin_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let cancelled_scroll = self.scroll_gesture.is_some();
        let started = self.begin_drag_snapshot(position);
        if started {
            if cancelled_scroll {
                self.schedule_ignored_scroll_recovery(cx);
            }
            cx.notify();
        }
        started
    }

    /// Updates the active pointer drag after locking it to the carousel axis.
    /// Cross-axis drags cancel this gesture so an ancestor can handle them.
    pub(super) fn update_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let Some(mut gesture) = self.pointer_gesture else {
            return false;
        };

        let delta = position - gesture.start_position;
        let primary_delta = self.primary_delta(delta);
        if !gesture.axis_locked {
            let cross_axis_delta = self.cross_axis_delta(delta);
            if primary_delta.abs().max(cross_axis_delta.abs()) <= POINTER_AXIS_LOCK_THRESHOLD {
                return false;
            }
            if cross_axis_delta.abs() > primary_delta.abs() {
                self.pointer_gesture = None;
                self.suppress_pointer_click = true;
                cx.notify();
                return false;
            }
            gesture.axis_locked = true;
        }

        gesture.total_delta = primary_delta;
        self.pointer_gesture = Some(gesture);
        let mut offset = gesture.start_offset;
        let next = self.clamped_offset(self.primary_offset(offset) + primary_delta);
        self.set_primary_offset(&mut offset, next);
        let changed = offset != self.scroll_handle.offset();
        self.scroll_handle.set_offset(offset);
        if changed {
            cx.notify();
        }
        true
    }

    /// Finishes a pointer drag by selecting the nearest item and settling the
    /// handle to its snap point.
    pub(super) fn finish_drag(&mut self, cx: &mut Context<Self>) -> bool {
        let suppressed_click = std::mem::take(&mut self.suppress_pointer_click);
        let Some(gesture) = self.pointer_gesture.take() else {
            if suppressed_click {
                cx.notify();
            }
            return false;
        };
        self.finish_snapshot(
            gesture.start_offset,
            gesture.start_index,
            gesture.total_delta,
            cx,
        )
    }

    /// Applies a precise trackpad delta and remembers the gesture's start.
    /// Returns whether the delta moved the handle or can be consumed by a
    /// looping carousel.
    pub(super) fn handle_scroll_delta(
        &mut self,
        axis: Axis,
        delta: Pixels,
        phase: TouchPhase,
        cx: &mut Context<Self>,
    ) -> bool {
        if axis != self.axis || self.item_count < 2 {
            return false;
        }

        if self.ignore_scroll_until_quiet {
            match phase {
                TouchPhase::Started => {
                    self.ignore_scroll_until_quiet = false;
                    self.invalidate_scroll_settle();
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.ignore_scroll_until_quiet = false;
                    self.invalidate_scroll_settle();
                    return false;
                }
                TouchPhase::Moved => {
                    self.schedule_ignored_scroll_recovery(cx);
                    return false;
                }
            }
        }

        if matches!(phase, TouchPhase::Started) && self.scroll_gesture.is_some() {
            self.finish_scroll(false, cx);
        }

        if self.scroll_gesture.is_none() {
            self.pointer_gesture = None;
            self.scroll_gesture = Some(ScrollGesture {
                start_offset: self.scroll_handle.offset(),
                start_index: self.selected_index,
                total_delta: px(0.),
            });
        }

        if let Some(gesture) = self.scroll_gesture.as_mut() {
            gesture.total_delta += delta;
        } else {
            return false;
        }

        let mut offset = self.scroll_handle.offset();
        let previous = offset;
        let next = self.clamped_offset(self.primary_offset(offset) + delta);
        self.set_primary_offset(&mut offset, next);
        self.scroll_handle.set_offset(offset);
        let moved = offset != previous;
        if moved {
            cx.notify();
        }
        if matches!(phase, TouchPhase::Started | TouchPhase::Moved) {
            self.schedule_scroll_settle(cx);
        }
        moved || self.looping
    }

    /// Finishes a precise trackpad gesture.  Cancelled gestures restore the
    /// original offset and never emit a selection event.
    pub(super) fn finish_scroll(&mut self, cancelled: bool, cx: &mut Context<Self>) -> bool {
        self.invalidate_scroll_settle();
        if self.ignore_scroll_until_quiet {
            self.ignore_scroll_until_quiet = false;
            return false;
        }
        let Some(gesture) = self.scroll_gesture.take() else {
            return false;
        };
        if cancelled {
            self.scroll_handle.set_offset(gesture.start_offset);
            cx.notify();
            return false;
        }

        self.finish_snapshot(
            gesture.start_offset,
            gesture.start_index,
            gesture.total_delta,
            cx,
        )
    }

    fn select_index_with_wrap(
        &mut self,
        index: usize,
        wrapped: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(index) = self.clamp_index(index) else {
            return false;
        };
        let was_interacting = self.is_interacting();
        self.cancel_interactions(cx);
        if self.selected_index == Some(index) {
            if was_interacting {
                cx.notify();
            }
            return false;
        }

        self.selected_index = Some(index);
        if wrapped {
            self.motion_revision = self.motion_revision.wrapping_add(1);
        }
        cx.emit(CarouselEvent::Change(index));
        cx.notify();
        true
    }

    fn clamp_index(&self, index: usize) -> Option<usize> {
        (self.item_count > 0).then_some(index.min(self.item_count - 1))
    }

    fn is_loop_wrap(&self, current: usize, next: usize) -> bool {
        self.looping
            && self.item_count > 1
            && ((current == 0 && next + 1 == self.item_count)
                || (current + 1 == self.item_count && next == 0))
    }

    fn cancel_interactions(&mut self, cx: &mut Context<Self>) {
        let cancelled_scroll = self.scroll_gesture.is_some();
        self.pointer_gesture = None;
        self.scroll_gesture = None;
        self.invalidate_scroll_settle();
        if cancelled_scroll {
            self.ignore_scroll_until_quiet = true;
            self.schedule_ignored_scroll_recovery(cx);
        }
    }

    fn cancel_user_interaction(&mut self, cx: &mut Context<Self>) {
        let was_interacting = self.is_interacting();
        self.cancel_interactions(cx);
        if was_interacting {
            cx.notify();
        }
    }

    fn begin_drag_snapshot(&mut self, position: Point<Pixels>) -> bool {
        self.suppress_pointer_click = false;
        if self.item_count < 2 {
            return false;
        }
        self.pointer_gesture = Some(PointerGesture {
            start_position: position,
            start_offset: self.scroll_handle.offset(),
            start_index: self.selected_index,
            total_delta: px(0.),
            axis_locked: false,
        });
        if self.scroll_gesture.is_some() {
            self.ignore_scroll_until_quiet = true;
        }
        self.scroll_gesture = None;
        self.invalidate_scroll_settle();
        true
    }

    fn schedule_scroll_settle(&mut self, cx: &mut Context<Self>) {
        self.scroll_settle_epoch = self.scroll_settle_epoch.wrapping_add(1);
        let epoch = self.scroll_settle_epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(SCROLL_EVENT_SEPARATION)
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |state, cx| {
                    if state.scroll_settle_epoch == epoch && state.scroll_gesture.is_some() {
                        state.finish_scroll(false, cx);
                    }
                });
            }
        })
        .detach();
    }

    fn schedule_ignored_scroll_recovery(&mut self, cx: &mut Context<Self>) {
        self.scroll_settle_epoch = self.scroll_settle_epoch.wrapping_add(1);
        let epoch = self.scroll_settle_epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(SCROLL_EVENT_SEPARATION)
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |state, _| {
                    if state.scroll_settle_epoch == epoch {
                        state.ignore_scroll_until_quiet = false;
                    }
                });
            }
        })
        .detach();
    }

    fn invalidate_scroll_settle(&mut self) {
        self.scroll_settle_epoch = self.scroll_settle_epoch.wrapping_add(1);
    }

    fn finish_snapshot(
        &mut self,
        start_offset: Point<Pixels>,
        start_index: Option<usize>,
        total_delta: Pixels,
        cx: &mut Context<Self>,
    ) -> bool {
        let current_offset = self.scroll_handle.offset();
        let selected = if self.looping {
            self.loop_boundary_index(start_index, total_delta)
                .or_else(|| self.nearest_index(current_offset))
        } else {
            self.nearest_index(current_offset)
        };

        let changed = selected.is_some_and(|index| {
            if self.selected_index == Some(index) {
                false
            } else {
                let wrapped = self
                    .selected_index
                    .zip(selected)
                    .is_some_and(|(current, next)| self.is_loop_wrap(current, next));
                self.select_index_with_wrap(index, wrapped, cx)
            }
        });

        if selected.is_none() {
            self.scroll_handle.set_offset(start_offset);
        }
        if !changed {
            cx.notify();
        }
        changed
    }

    fn loop_boundary_index(
        &self,
        start_index: Option<usize>,
        total_delta: Pixels,
    ) -> Option<usize> {
        let start_index = start_index?;
        let threshold = self.snap_extent() * 0.25;
        if total_delta.abs() < threshold.max(px(1.)) {
            return None;
        }
        if start_index == 0 && total_delta > px(0.) {
            self.item_count.checked_sub(1)
        } else if start_index + 1 == self.item_count && total_delta < px(0.) {
            Some(0)
        } else {
            None
        }
    }

    fn snap_extent(&self) -> Pixels {
        self.geometry
            .viewport
            .map(|bounds| bounds.size.along(self.axis))
            .or_else(|| {
                self.geometry
                    .items
                    .first()
                    .map(|bounds| bounds.size.along(self.axis))
            })
            .unwrap_or(px(1.))
    }

    fn snap_offset(&self, viewport: Bounds<Pixels>, item: Bounds<Pixels>) -> Point<Pixels> {
        let mut offset = self.scroll_handle.offset();
        let target = match self.axis {
            Axis::Horizontal => viewport.left() - item.left(),
            Axis::Vertical => viewport.top() - item.top(),
        }
        .clamp(-self.max_snap_offset(), px(0.));
        self.set_primary_offset(&mut offset, target);
        offset
    }

    fn max_snap_offset(&self) -> Pixels {
        let handle_max = self
            .primary_offset(self.scroll_handle.max_offset())
            .max(px(0.));
        let Some(viewport) = self.geometry.viewport else {
            return handle_max;
        };
        let Some(first) = self.geometry.items.first() else {
            return handle_max;
        };

        let (mut content_start, mut content_end) = match self.axis {
            Axis::Horizontal => (first.left(), first.right()),
            Axis::Vertical => (first.top(), first.bottom()),
        };
        for item in &self.geometry.items[1..] {
            let (start, end) = match self.axis {
                Axis::Horizontal => (item.left(), item.right()),
                Axis::Vertical => (item.top(), item.bottom()),
            };
            content_start = content_start.min(start);
            content_end = content_end.max(end);
        }
        let geometry_max =
            (content_end - content_start - viewport.size.along(self.axis)).max(px(0.));
        handle_max.max(geometry_max)
    }

    fn primary_distance(
        &self,
        offset: Point<Pixels>,
        viewport: Bounds<Pixels>,
        item: Bounds<Pixels>,
    ) -> Pixels {
        let target = self.snap_offset(viewport, item);
        (self.primary_offset(offset) - self.primary_offset(target)).abs()
    }

    fn primary_offset(&self, offset: Point<Pixels>) -> Pixels {
        match self.axis {
            Axis::Horizontal => offset.x,
            Axis::Vertical => offset.y,
        }
    }

    fn set_primary_offset(&self, offset: &mut Point<Pixels>, value: Pixels) {
        match self.axis {
            Axis::Horizontal => offset.x = value,
            Axis::Vertical => offset.y = value,
        }
    }

    fn primary_delta(&self, delta: Point<Pixels>) -> Pixels {
        match self.axis {
            Axis::Horizontal => delta.x,
            Axis::Vertical => delta.y,
        }
    }

    fn cross_axis_delta(&self, delta: Point<Pixels>) -> Pixels {
        match self.axis {
            Axis::Horizontal => delta.y,
            Axis::Vertical => delta.x,
        }
    }

    fn clamped_offset(&self, value: Pixels) -> Pixels {
        let max_offset = self.scroll_handle.max_offset();
        let bound = self.primary_offset(max_offset).max(px(0.));
        value.clamp(-bound, px(0.))
    }
}

impl EventEmitter<CarouselEvent> for CarouselState {}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext as _, TestAppContext, point, px};

    use super::*;

    #[test]
    fn constructors_and_programmatic_setters_clamp_without_events() {
        let state = CarouselState::new(3)
            .with_selected_index(99)
            .with_axis(Axis::Vertical)
            .with_looping(true);
        assert_eq!(state.item_count(), 3);
        assert_eq!(state.selected_index(), Some(2));
        assert_eq!(state.axis(), Axis::Vertical);
        assert!(state.is_looping());
        assert!(state.has_previous());
        assert!(state.has_next());

        let empty = CarouselState::new(0);
        assert_eq!(empty.selected_index(), None);
        assert!(!empty.has_previous());
        assert!(!empty.has_next());
    }

    #[test]
    fn geometry_produces_axis_specific_snap_points() {
        let mut state = CarouselState::new(2).with_axis(Axis::Horizontal);
        state.set_geometry(
            Bounds::new(point(px(10.), px(20.)), gpui::size(px(100.), px(40.))),
            vec![
                Bounds::new(point(px(10.), px(20.)), gpui::size(px(100.), px(40.))),
                Bounds::new(point(px(110.), px(20.)), gpui::size(px(100.), px(40.))),
            ],
        );
        assert_eq!(state.snap_target_for(0), Some(point(px(0.), px(0.))));
        assert_eq!(state.snap_target_for(1), Some(point(px(-100.), px(0.))));
        let mut vertical = CarouselState::new(2).with_axis(Axis::Vertical);
        vertical.set_geometry(
            Bounds::new(point(px(10.), px(20.)), gpui::size(px(40.), px(100.))),
            vec![
                Bounds::new(point(px(10.), px(20.)), gpui::size(px(40.), px(100.))),
                Bounds::new(point(px(10.), px(120.)), gpui::size(px(40.), px(100.))),
            ],
        );
        assert_eq!(vertical.snap_target_for(1), Some(point(px(0.), px(-100.))));

        let mut narrow = CarouselState::new(3);
        narrow.set_geometry(
            Bounds::new(point(px(0.), px(0.)), gpui::size(px(100.), px(40.))),
            vec![
                Bounds::new(point(px(0.), px(0.)), gpui::size(px(50.), px(40.))),
                Bounds::new(point(px(50.), px(0.)), gpui::size(px(50.), px(40.))),
                Bounds::new(point(px(100.), px(0.)), gpui::size(px(50.), px(40.))),
            ],
        );
        assert_eq!(narrow.snap_target_for(2), Some(point(px(-50.), px(0.))));
    }

    #[gpui::test]
    fn non_looping_navigation_stops_at_both_boundaries(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(3)));
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &CarouselEvent, _| {
                let CarouselEvent::Change(index) = event;
                events.borrow_mut().push(*index);
            })
        });

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                assert!(!state.select_previous(cx));
                assert!(!state.select_first(cx));
                assert!(state.select_last(cx));
                assert!(!state.select_next(cx));
                assert!(!state.select_last(cx));
            });
        });

        assert_eq!(events.borrow().as_slice(), &[2]);
    }

    #[gpui::test]
    fn user_navigation_emits_once_and_programmatic_changes_stay_silent(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(3)));
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &CarouselEvent, _| {
                let CarouselEvent::Change(index) = event;
                events.borrow_mut().push(*index);
            })
        });

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                assert!(state.select_next(cx));
                assert!(!state.select_index(1, cx));
                assert!(!state.select_index(99, cx));
                state.set_selected_index(2, cx);
                state.set_axis(Axis::Vertical, cx);
                state.set_looping(true, cx);
            });
        });

        assert_eq!(events.borrow().as_slice(), &[1]);
        assert_eq!(
            state.read_with(cx, |state, _| state.selected_index()),
            Some(2)
        );
        assert_eq!(state.read_with(cx, |state, _| state.axis()), Axis::Vertical);
        assert!(state.read_with(cx, |state, _| state.is_looping()));

        let empty = cx.update(|cx| cx.new(|_| CarouselState::new(0)));
        cx.update(|cx| {
            empty.update(cx, |state, cx| {
                state.set_item_count(2, cx);
                state.set_selected_index(99, cx);
            });
        });
        assert_eq!(
            empty.read_with(cx, |state, _| state.selected_index()),
            Some(1)
        );
    }

    #[gpui::test]
    fn pointer_drag_locks_to_the_primary_axis(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(3)));

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                assert!(state.begin_drag(point(px(0.), px(0.)), cx));
                assert!(!state.update_drag(point(px(1.), px(1.)), cx));
                assert!(state.is_interacting());
                assert!(!state.is_pointer_drag_locked());
                assert!(!state.update_drag(point(px(4.), px(20.)), cx));
                assert!(!state.is_interacting());
                assert!(state.should_suppress_pointer_click());
                assert!(!state.finish_drag(cx));
                assert!(!state.should_suppress_pointer_click());

                assert!(state.begin_drag(point(px(0.), px(0.)), cx));
                assert!(state.update_drag(point(px(20.), px(4.)), cx));
                assert!(state.is_interacting());
                assert!(state.is_pointer_drag_locked());
                state.finish_drag(cx);
                assert!(!state.is_interacting());
            });
        });
    }

    #[gpui::test]
    fn finishing_a_drag_keeps_the_offset_as_the_snap_animation_origin(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                state.set_geometry(
                    Bounds::new(point(px(0.), px(0.)), gpui::size(px(100.), px(40.))),
                    vec![
                        Bounds::new(point(px(0.), px(0.)), gpui::size(px(100.), px(40.))),
                        Bounds::new(point(px(100.), px(0.)), gpui::size(px(100.), px(40.))),
                    ],
                );
                assert!(state.begin_drag(point(px(0.), px(0.)), cx));
                state.scroll_handle.set_offset(point(px(-60.), px(0.)));
                assert!(state.finish_drag(cx));
                assert_eq!(state.selected_index(), Some(1));
                assert_eq!(state.scroll_handle.offset(), point(px(-60.), px(0.)));
            });
        });
    }

    #[gpui::test]
    fn user_navigation_invalidates_an_active_trackpad_gesture(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(3)));
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &CarouselEvent, _| {
                let CarouselEvent::Change(index) = event;
                events.borrow_mut().push(*index);
            })
        });

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                state.handle_scroll_delta(Axis::Horizontal, px(-20.), TouchPhase::Started, cx);
                assert!(state.is_interacting());
                assert!(state.select_next(cx));
                assert!(!state.is_interacting());
                assert!(!state.handle_scroll_delta(
                    Axis::Horizontal,
                    px(-20.),
                    TouchPhase::Moved,
                    cx,
                ));
                assert!(!state.is_interacting());
                assert!(state.ignore_scroll_until_quiet);
            });
        });

        cx.run_until_parked();
        cx.executor().advance_clock(SCROLL_EVENT_SEPARATION);
        cx.run_until_parked();

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                assert!(!state.ignore_scroll_until_quiet);
                assert!(!state.handle_scroll_delta(
                    Axis::Horizontal,
                    px(-20.),
                    TouchPhase::Moved,
                    cx,
                ));
                assert!(state.is_interacting());
                assert!(!state.finish_scroll(true, cx));
                assert!(!state.is_interacting());

                assert!(!state.handle_scroll_delta(
                    Axis::Horizontal,
                    px(-20.),
                    TouchPhase::Started,
                    cx,
                ));
                assert!(state.is_interacting());
                assert!(!state.finish_scroll(true, cx));
                assert!(!state.is_interacting());
            });
        });

        assert_eq!(events.borrow().as_slice(), &[1]);
    }

    #[gpui::test]
    fn moved_only_trackpad_gesture_settles_after_the_quiet_period(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &CarouselEvent, _| {
                let CarouselEvent::Change(index) = event;
                events.borrow_mut().push(*index);
            })
        });

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                state.set_geometry(
                    Bounds::new(point(px(0.), px(0.)), gpui::size(px(100.), px(40.))),
                    vec![
                        Bounds::new(point(px(0.), px(0.)), gpui::size(px(100.), px(40.))),
                        Bounds::new(point(px(100.), px(0.)), gpui::size(px(100.), px(40.))),
                    ],
                );
                state.handle_scroll_delta(Axis::Horizontal, px(-60.), TouchPhase::Moved, cx);
                state.scroll_handle.set_offset(point(px(-60.), px(0.)));
                assert!(state.is_interacting());
            });
        });

        cx.run_until_parked();
        cx.executor().advance_clock(SCROLL_EVENT_SEPARATION);
        cx.run_until_parked();

        assert_eq!(
            state.read_with(cx, |state, _| state.selected_index()),
            Some(1)
        );
        assert!(!state.read_with(cx, |state, _| state.is_interacting()));
        assert_eq!(events.borrow().as_slice(), &[1]);
    }

    #[gpui::test]
    fn invalid_boundary_navigation_still_cancels_the_active_gesture(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2)));

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                state.handle_scroll_delta(Axis::Horizontal, px(20.), TouchPhase::Started, cx);
                assert!(state.is_interacting());
                assert!(!state.select_previous(cx));
                assert!(!state.is_interacting());
                assert!(!state.handle_scroll_delta(
                    Axis::Horizontal,
                    px(20.),
                    TouchPhase::Moved,
                    cx,
                ));
                assert!(!state.finish_scroll(false, cx));

                assert!(state.select_last(cx));
                state.handle_scroll_delta(Axis::Horizontal, px(-20.), TouchPhase::Started, cx);
                assert!(state.is_interacting());
                assert!(!state.select_next(cx));
                assert!(!state.is_interacting());
                assert!(!state.finish_scroll(false, cx));
            });
        });
    }

    #[gpui::test]
    fn looping_boundaries_emit_one_event_and_advance_motion_revision(cx: &mut TestAppContext) {
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(2).with_looping(true)));
        let events = Rc::new(RefCell::new(Vec::new()));
        let _subscription = cx.update(|cx| {
            let events = events.clone();
            cx.subscribe(&state, move |_, event: &CarouselEvent, _| {
                let CarouselEvent::Change(index) = event;
                events.borrow_mut().push(*index);
            })
        });

        cx.update(|cx| {
            state.update(cx, |state, cx| {
                assert!(state.select_previous(cx));
                assert_eq!(state.selected_index(), Some(1));
                assert_eq!(state.motion_revision(), 1);
                assert!(state.select_next(cx));
                assert_eq!(state.selected_index(), Some(0));
                assert_eq!(state.motion_revision(), 2);
            });
        });
        assert_eq!(events.borrow().as_slice(), &[1, 0]);
    }
}
