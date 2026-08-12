use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use gpui::{
    Anchor, Animation, AnimationExt as _, AnyElement, App, Div, ElementId, InteractiveElement,
    Interactivity, IntoElement, ParentElement, Pixels, RenderOnce, Role, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
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
    heights: Rc<RefCell<Vec<Pixels>>>,
    expanded: Rc<std::cell::Cell<bool>>,
}

/// A deep toast-stack element that owns measurement, overlap, and expansion motion.
#[derive(IntoElement)]
pub struct ToastStack {
    base: Stateful<Div>,
    style: StyleRefinement,
    state: ToastStackState,
    motion: ToastMotion,
    placement: Anchor,
    children: Vec<AnyElement>,
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
            children: Vec::new(),
        }
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
}

impl Styled for ToastStack {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ToastStack {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
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
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let expanded = self.state.expanded.get();
        let heights = self.state.heights.borrow().clone();
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
            .map(move |(index, child)| {
                let (collapsed_offset, expanded_offset) =
                    offsets.get(index).copied().unwrap_or((px(0.), px(0.)));
                let measured = measured.clone();
                div()
                    .id(("base-toast-stack-item", index))
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .with_animation(
                        ElementId::NamedInteger(
                            "base-toast-stack-layout".into(),
                            ((index as u64) << 1) | u64::from(expanded),
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
                        if heights.len() <= index {
                            heights.resize(index + 1, px(0.));
                        }
                        if heights[index] != bounds.size.height {
                            heights[index] = bounds.size.height;
                            cx.refresh_windows();
                        }
                    })
                    .child(child)
            });

        let expanded_state = self.state.expanded.clone();
        self.base
            .relative()
            .h(stack_height)
            .on_hover(move |hovered, _, cx| {
                if expanded_state.replace(*hovered) != *hovered {
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

/// Behavior state for a toast that remains mounted through its exit transition.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToastLifecycle {
    status: ToastTransitionStatus,
}

impl ToastLifecycle {
    /// Create lifecycle state for a newly mounted toast.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the current transition phase.
    pub fn status(&self) -> ToastTransitionStatus {
        self.status
    }

    /// Mark the enter transition as complete.
    pub fn finish_enter(&mut self) {
        if self.status == ToastTransitionStatus::Starting {
            self.status = ToastTransitionStatus::Present;
        }
    }

    /// Begin closing and return whether an exit transition should be started.
    pub fn close(&mut self) -> bool {
        if self.status == ToastTransitionStatus::Ending {
            return false;
        }
        self.status = ToastTransitionStatus::Ending;
        true
    }
}

/// Interaction state shared by a toast viewport and its presentation layer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToastViewportState {
    expanded: bool,
}

impl ToastViewportState {
    /// Create collapsed viewport interaction state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether the application should present the toast stack expanded.
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    /// Return whether the toast stack is expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
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

/// An unstyled viewport for application-rendered toast roots.
#[derive(IntoElement)]
pub struct ToastViewport {
    base: Stateful<Div>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl ToastViewport {
    /// Create an unstyled toast viewport.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Styled for ToastViewport {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ToastViewport {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}

impl InteractiveElement for ToastViewport {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ToastViewport {}

impl RenderOnce for ToastViewport {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.children(self.children).refine_style(&self.style)
    }
}

/// Ordered toast storage with Base UI-compatible unique-id replacement semantics.
#[derive(Debug)]
pub struct ToastStore<I, T> {
    entries: VecDeque<(I, T)>,
}

impl<I, T> Default for ToastStore<I, T> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

impl<I: Eq, T> ToastStore<I, T> {
    /// Adds an item at the end, replacing an existing item with the same id.
    pub fn push(&mut self, id: I, item: T) -> Option<T> {
        let replaced = self.remove(&id);
        self.entries.push_back((id, item));
        replaced
    }

    /// Remove and return the item with the given id.
    pub fn remove(&mut self, id: &I) -> Option<T> {
        let ix = self
            .entries
            .iter()
            .position(|(entry_id, _)| entry_id == id)?;
        self.entries.remove(ix).map(|(_, item)| item)
    }

    /// Return the item with the given id.
    pub fn get(&self, id: &I) -> Option<&T> {
        self.entries
            .iter()
            .find_map(|(entry_id, item)| (entry_id == id).then_some(item))
    }

    /// Iterate over ids and values in display order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&I, &T)> {
        self.entries.iter().map(|(id, item)| (id, item))
    }

    /// Iterate over values in display order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.entries.iter().map(|(_, item)| item)
    }

    /// Iterate over the newest `limit` values while preserving display order.
    pub fn visible_values(&self, limit: usize) -> impl DoubleEndedIterator<Item = &T> {
        self.entries
            .iter()
            .skip(self.entries.len().saturating_sub(limit))
            .map(|(_, item)| item)
    }

    /// Remove every stored toast.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Return the number of stored toasts.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the store has no toasts.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Element as _, accesskit};

    #[test]
    fn store_replaces_an_existing_id_at_the_newest_position() {
        let mut store = ToastStore::default();
        assert_eq!(store.push("a", 1), None);
        assert_eq!(store.push("b", 2), None);
        assert_eq!(store.push("a", 3), Some(1));
        assert_eq!(
            store
                .iter()
                .map(|(id, value)| (*id, *value))
                .collect::<Vec<_>>(),
            vec![("b", 2), ("a", 3)]
        );
    }

    #[test]
    fn lifecycle_closes_only_once() {
        let mut lifecycle = ToastLifecycle::new();
        assert_eq!(lifecycle.status(), ToastTransitionStatus::Starting);
        lifecycle.finish_enter();
        assert_eq!(lifecycle.status(), ToastTransitionStatus::Present);
        assert!(lifecycle.close());
        assert!(!lifecycle.close());
        assert_eq!(lifecycle.status(), ToastTransitionStatus::Ending);
    }

    #[test]
    fn visible_values_keeps_the_newest_items_in_display_order() {
        let mut store = ToastStore::default();
        store.push("a", 1);
        store.push("b", 2);
        store.push("c", 3);
        assert_eq!(
            store.visible_values(2).copied().collect::<Vec<_>>(),
            vec![2, 3]
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
