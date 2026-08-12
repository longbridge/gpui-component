use std::collections::VecDeque;

use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    RenderOnce, Role, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
};

use crate::StyledExt as _;

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
