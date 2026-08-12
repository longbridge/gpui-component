use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    Anchor, Animation, AnimationExt as _, AnyElement, App, Div, ElementId, FocusHandle,
    InteractiveElement, Interactivity, IntoElement, ParentElement, Pixels, RenderOnce, Role,
    Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};

use crate::{ElementExt as _, StyledExt as _, animation::cubic_bezier};

/// Motion tokens used by an unstyled toast stack.
#[derive(Clone, Copy, Debug)]
pub struct ToastMotion {
    /// Duration of stack expansion and collapse.
    pub duration: Duration,
    /// Visible distance between collapsed toast layers.
    pub collapsed_peek: Pixels,
    /// Distance between expanded toast items.
    pub expanded_gap: Pixels,
}

impl ToastMotion {
    /// Create motion matching the Base UI Toast example.
    pub fn base_ui() -> Self {
        Self {
            duration: Duration::from_millis(500),
            collapsed_peek: px(12.),
            expanded_gap: px(12.),
        }
    }
}

impl Default for ToastMotion {
    fn default() -> Self {
        Self::base_ui()
    }
}

/// Persistent private layout state used by [`ToastStack`].
#[derive(Clone, Debug, Default)]
pub struct ToastStackState {
    heights: Rc<RefCell<std::collections::HashMap<ElementId, Pixels>>>,
    hovered: Rc<std::cell::Cell<bool>>,
    focused: Rc<std::cell::Cell<bool>>,
}

impl ToastStackState {
    /// Return whether interaction has expanded the stack.
    pub fn is_expanded(&self) -> bool {
        self.hovered.get() || self.focused.get()
    }
}

/// Options applied when a toast enters a [`ToastManager`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ToastOptions {
    /// Active time before automatic dismissal; `None` disables auto-hide.
    pub timeout: Option<Duration>,
}

#[derive(Debug)]
struct ManagedToast<I, T> {
    id: I,
    value: T,
    status: ToastTransitionStatus,
    timeout_remaining: Option<Duration>,
    transition_elapsed: Duration,
}

/// Changes produced when a toast manager advances its lifecycle clock.
#[derive(Debug)]
pub struct ToastAdvance<I, T> {
    /// Toast ids that entered their ending transition.
    pub ending: Vec<I>,
    /// Toast values removed after their ending transition completed.
    pub removed: Vec<(I, T)>,
}

/// Ordered toast storage, lifecycle, auto-hide, limits, and exit coordination.
#[derive(Debug)]
pub struct ToastManager<I, T> {
    entries: VecDeque<ManagedToast<I, T>>,
    last_advance: Option<Instant>,
    transition_duration: Duration,
}

impl<I, T> ToastManager<I, T> {
    /// Create a manager using the supplied motion duration for enter and exit.
    pub fn new(motion: ToastMotion) -> Self {
        Self {
            entries: VecDeque::new(),
            last_advance: None,
            transition_duration: motion.duration,
        }
    }

    /// Return the number of mounted toasts, including ending toasts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether no toast is mounted.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over mounted toast ids, values, and phases in display order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&I, &T, ToastTransitionStatus)> {
        self.entries
            .iter()
            .map(|entry| (&entry.id, &entry.value, entry.status))
    }

    /// Iterate over newest visible active toasts plus ending toasts.
    pub fn visible(&self, limit: usize) -> impl Iterator<Item = (&I, &T, ToastTransitionStatus)> {
        let first = self
            .entries
            .iter()
            .filter(|entry| entry.status != ToastTransitionStatus::Ending)
            .count()
            .saturating_sub(limit);
        let mut active_index = 0usize;
        self.entries.iter().filter_map(move |entry| {
            let visible = if entry.status == ToastTransitionStatus::Ending {
                true
            } else {
                let keep = active_index >= first;
                active_index += 1;
                keep
            };
            visible.then_some((&entry.id, &entry.value, entry.status))
        })
    }

    /// Return a mounted toast value by id.
    pub fn get(&self, id: &I) -> Option<&T>
    where
        I: Eq,
    {
        self.entries
            .iter()
            .find_map(|entry| (&entry.id == id).then_some(&entry.value))
    }
}

impl<I: Clone + Eq, T> ToastManager<I, T> {
    /// Add a newest toast, replacing an existing toast with the same id.
    pub fn push(&mut self, id: I, value: T, options: ToastOptions, now: Instant) -> Option<T> {
        if self.entries.is_empty() {
            self.last_advance = Some(now);
        }
        let replaced = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .and_then(|index| self.entries.remove(index))
            .map(|entry| entry.value);
        self.entries.push_back(ManagedToast {
            id,
            value,
            status: ToastTransitionStatus::Starting,
            timeout_remaining: options.timeout,
            transition_elapsed: Duration::ZERO,
        });
        self.last_advance.get_or_insert(now);
        replaced
    }

    /// Begin a toast's exit transition, returning whether its state changed.
    pub fn dismiss(&mut self, id: &I, now: Instant) -> bool {
        let delta = self
            .last_advance
            .replace(now)
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        for entry in &mut self.entries {
            if entry.status == ToastTransitionStatus::Ending {
                entry.transition_elapsed += delta;
            }
        }
        let Some(entry) = self.entries.iter_mut().find(|entry| &entry.id == id) else {
            return false;
        };
        if entry.status == ToastTransitionStatus::Ending {
            return false;
        }
        entry.status = ToastTransitionStatus::Ending;
        entry.transition_elapsed = Duration::ZERO;
        true
    }

    /// Begin the exit transition for every active toast.
    pub fn dismiss_all(&mut self, now: Instant) -> Vec<I> {
        let delta = self
            .last_advance
            .replace(now)
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        for entry in &mut self.entries {
            if entry.status == ToastTransitionStatus::Ending {
                entry.transition_elapsed += delta;
            }
        }
        let mut changed = Vec::new();
        for entry in &mut self.entries {
            if entry.status != ToastTransitionStatus::Ending {
                entry.status = ToastTransitionStatus::Ending;
                entry.transition_elapsed = Duration::ZERO;
                changed.push(entry.id.clone());
            }
        }
        changed
    }

    /// Advance lifecycle time; active timers pause while `paused` is true.
    pub fn advance(&mut self, now: Instant, paused: bool) -> ToastAdvance<I, T> {
        let delta = self
            .last_advance
            .replace(now)
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        let mut ending = Vec::new();
        for entry in &mut self.entries {
            match entry.status {
                ToastTransitionStatus::Starting => {
                    entry.transition_elapsed += delta;
                    if entry.transition_elapsed >= self.transition_duration {
                        entry.status = ToastTransitionStatus::Present;
                        entry.transition_elapsed = Duration::ZERO;
                    }
                }
                ToastTransitionStatus::Present if !paused => {
                    if let Some(remaining) = &mut entry.timeout_remaining {
                        *remaining = remaining.saturating_sub(delta);
                        if remaining.is_zero() {
                            entry.status = ToastTransitionStatus::Ending;
                            entry.transition_elapsed = Duration::ZERO;
                            ending.push(entry.id.clone());
                        }
                    }
                }
                ToastTransitionStatus::Ending => entry.transition_elapsed += delta,
                ToastTransitionStatus::Present => {}
            }
        }
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.entries.len() {
            if self.entries[index].status == ToastTransitionStatus::Ending
                && self.entries[index].transition_elapsed >= self.transition_duration
            {
                let entry = self.entries.remove(index).expect("toast index is valid");
                removed.push((entry.id, entry.value));
            } else {
                index += 1;
            }
        }
        if self.entries.is_empty() {
            self.last_advance = None;
        }
        ToastAdvance { ending, removed }
    }
}

/// A deep toast-stack element that owns measurement, overlap, and expansion motion.
#[derive(IntoElement)]
pub struct ToastStack {
    base: Stateful<Div>,
    style: StyleRefinement,
    state: ToastStackState,
    motion: ToastMotion,
    placement: Anchor,
    focus_handle: Option<FocusHandle>,
    children: Vec<(ElementId, AnyElement)>,
}

impl ToastStack {
    /// Create a toast stack with Base UI-compatible motion.
    pub fn new(id: impl Into<ElementId>, state: ToastStackState) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            state,
            motion: ToastMotion::base_ui(),
            placement: Anchor::TopRight,
            focus_handle: None,
            children: Vec::new(),
        }
    }

    /// Add a stably keyed toast item to the stack.
    pub fn item(mut self, id: impl Into<ElementId>, child: impl IntoElement) -> Self {
        self.children.push((id.into(), child.into_any_element()));
        self
    }

    /// Set the stack motion tokens.
    pub fn motion(mut self, motion: ToastMotion) -> Self {
        self.motion = motion;
        self
    }

    /// Set the viewport edge used to anchor stack geometry.
    pub fn placement(mut self, placement: Anchor) -> Self {
        self.placement = placement;
        self
    }

    /// Set the focus scope that expands the stack and pauses auto-hide timers.
    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }
}

impl Styled for ToastStack {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ToastStack {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(children.into_iter().enumerate().map(|(index, child)| {
                (
                    ElementId::NamedInteger("toast-stack-child".into(), index as u64),
                    child,
                )
            }));
    }
}

impl InteractiveElement for ToastStack {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ToastStack {}

fn stack_geometry(
    heights: &[Pixels],
    gap: Pixels,
    peek: Pixels,
    anchored_bottom: bool,
) -> (Pixels, Pixels, Vec<(Pixels, Pixels)>) {
    let count = heights.len();
    let expanded_height = heights
        .iter()
        .copied()
        .fold(px(0.), |sum, height| sum + height)
        + gap * count.saturating_sub(1) as f32;
    let front_height = heights.last().copied().unwrap_or(px(0.));
    let collapsed_height = front_height + peek * count.saturating_sub(1) as f32;
    let offsets = heights
        .iter()
        .enumerate()
        .map(|(index, height)| {
            let rank = count - 1 - index;
            let newer_height = heights[(index + 1)..]
                .iter()
                .copied()
                .fold(px(0.), |sum, height| sum + height);
            let expanded = if anchored_bottom {
                expanded_height - newer_height - gap * rank as f32 - *height
            } else {
                newer_height + gap * rank as f32
            };
            let collapsed = if anchored_bottom {
                collapsed_height - front_height - peek * rank as f32
            } else {
                front_height + peek * rank as f32 - *height
            };
            (collapsed, expanded)
        })
        .collect();
    (collapsed_height, expanded_height, offsets)
}

impl RenderOnce for ToastStack {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focused = self
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.contains_focused(window, cx));
        self.state.focused.set(focused);
        let expanded = self.state.is_expanded();
        let measured_by_id = self.state.heights.borrow().clone();
        let keys = self
            .children
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let heights = keys
            .iter()
            .map(|id| measured_by_id.get(id).copied().unwrap_or(px(0.)))
            .collect::<Vec<_>>();
        let measured = self.state.heights.clone();
        let duration = self.motion.duration;
        let peek = self.motion.collapsed_peek;
        let gap = self.motion.expanded_gap;
        let count = self.children.len();
        let anchored_bottom = matches!(
            self.placement,
            Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight
        );
        let (collapsed_height, expanded_height, offsets) = stack_geometry(
            &heights[..count.min(heights.len())],
            gap,
            peek,
            anchored_bottom,
        );
        let stack_height = if expanded {
            expanded_height
        } else {
            collapsed_height
        };
        let items = self
            .children
            .into_iter()
            .enumerate()
            .map(move |(index, (item_id, child))| {
                let (collapsed_offset, expanded_offset) =
                    offsets.get(index).copied().unwrap_or((px(0.), px(0.)));
                let measured = measured.clone();
                let measured_id = item_id.clone();
                div()
                    .id(item_id.clone())
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .with_animation(
                        ElementId::NamedInteger(
                            format!("base-toast-stack-layout-{item_id:?}").into(),
                            u64::from(expanded),
                        ),
                        Animation::new(duration).with_easing(cubic_bezier(0.22, 1., 0.36, 1.)),
                        move |this, delta| {
                            let offset = if expanded {
                                collapsed_offset + (expanded_offset - collapsed_offset) * delta
                            } else {
                                expanded_offset + (collapsed_offset - expanded_offset) * delta
                            };
                            this.top(offset)
                        },
                    )
                    .on_prepaint(move |bounds, _, cx| {
                        let mut heights = measured.borrow_mut();
                        if heights.get(&measured_id).copied() != Some(bounds.size.height) {
                            heights.insert(measured_id.clone(), bounds.size.height);
                            cx.refresh_windows();
                        }
                    })
                    .child(child)
            });

        let hovered_state = self.state.hovered.clone();
        self.base
            .relative()
            .h(stack_height)
            .when_some(self.focus_handle, |this, handle| this.track_focus(&handle))
            .on_hover(move |hovered, _, cx| {
                if hovered_state.replace(*hovered) != *hovered {
                    cx.refresh_windows();
                }
            })
            .children(items)
            .refine_style(&self.style)
    }
}

/// The lifecycle phase exposed by a toast root to application-owned presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToastTransitionStatus {
    /// The toast has just been added and may run its enter transition.
    #[default]
    Starting,
    /// The toast is fully present.
    Present,
    /// The toast is closing and remains mounted until its exit transition completes.
    Ending,
}

/// An unstyled semantic toast root. Applications own all presentation and motion.
#[derive(IntoElement)]
pub struct Toast {
    base: Stateful<Div>,
    style: StyleRefinement,
    transition_status: ToastTransitionStatus,
    children: Vec<AnyElement>,
}

impl Toast {
    /// Create an unstyled semantic toast in the starting transition phase.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            transition_status: ToastTransitionStatus::Starting,
            children: Vec::new(),
        }
    }

    /// Set the lifecycle phase used by application-owned toast presentation.
    pub fn transition_status(mut self, status: ToastTransitionStatus) -> Self {
        self.transition_status = status;
        self
    }

    /// Return the lifecycle phase used by application-owned toast presentation.
    pub fn status(&self) -> ToastTransitionStatus {
        self.transition_status
    }
}

impl Styled for Toast {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Toast {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl InteractiveElement for Toast {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Toast {}

impl RenderOnce for Toast {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .role(Role::Alert)
            .children(self.children)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element as _, accesskit};

    #[test]
    fn manager_pauses_timeout_and_removes_only_after_exit() {
        let motion = ToastMotion::base_ui();
        let start = Instant::now();
        let mut manager = ToastManager::new(motion);
        manager.push(
            "a",
            1,
            ToastOptions {
                timeout: Some(Duration::from_secs(5)),
            },
            start,
        );
        manager.advance(start + motion.duration, false);
        manager.advance(start + motion.duration + Duration::from_secs(4), true);
        assert_eq!(
            manager.iter().next().unwrap().2,
            ToastTransitionStatus::Present
        );
        let ending = manager.advance(start + motion.duration + Duration::from_secs(9), false);
        assert_eq!(ending.ending, vec!["a"]);
        assert!(ending.removed.is_empty());
        let removed = manager.advance(start + motion.duration * 2 + Duration::from_secs(9), false);
        assert_eq!(removed.removed, vec![("a", 1)]);
    }

    #[test]
    fn manager_limit_keeps_ending_toasts_mounted() {
        let now = Instant::now();
        let mut manager = ToastManager::new(ToastMotion::base_ui());
        for id in ["a", "b", "c"] {
            manager.push(id, id, ToastOptions::default(), now);
        }
        manager.dismiss(&"a", now);
        assert_eq!(
            manager.visible(1).map(|(id, _, _)| *id).collect::<Vec<_>>(),
            vec!["a", "c"]
        );
    }

    #[test]
    fn manager_resets_clock_when_reused_after_becoming_empty() {
        let motion = ToastMotion::base_ui();
        let start = Instant::now();
        let mut manager = ToastManager::new(motion);
        manager.push("old", 1, ToastOptions::default(), start);
        manager.dismiss(&"old", start);
        manager.advance(start + motion.duration, false);
        let later = start + Duration::from_secs(60);
        manager.push("new", 2, ToastOptions::default(), later);
        manager.advance(later + Duration::from_millis(50), false);
        assert_eq!(
            manager.iter().next().unwrap().2,
            ToastTransitionStatus::Starting
        );
    }

    #[test]
    fn stack_geometry_anchors_newest_item_and_supports_variable_heights() {
        let heights = [px(40.), px(60.), px(80.)];
        let (collapsed, expanded, top) = stack_geometry(&heights, px(12.), px(12.), false);
        assert_eq!(collapsed, px(104.));
        assert_eq!(expanded, px(204.));
        assert_eq!(
            top,
            vec![(px(64.), px(164.)), (px(32.), px(92.)), (px(0.), px(0.))]
        );

        let (_, _, bottom) = stack_geometry(&heights, px(12.), px(12.), true);
        assert_eq!(
            bottom,
            vec![(px(0.), px(0.)), (px(12.), px(52.)), (px(24.), px(124.))]
        );
    }

    #[gpui::test]
    fn toast_exposes_alert_semantics(cx: &mut gpui::TestAppContext) {
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let mut node = accesskit::Node::new(Role::Alert);
            Toast::new("toast")
                .render(window, cx)
                .into_element()
                .write_a11y_info(&mut node);
            assert_eq!(node.role(), Role::Alert);
        });
    }
}
