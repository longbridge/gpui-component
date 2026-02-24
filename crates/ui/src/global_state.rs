use gpui::{App, ElementId, Entity, FocusHandle, Global};
use std::collections::HashSet;

use crate::text::TextViewState;

pub(crate) fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}

impl Global for GlobalState {}

pub(crate) struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
    /// Set of open popover IDs that use deferred rendering.
    /// When this set is not empty, we are inside at least one deferred context.
    /// This is used to prevent double-deferred elements which would cause GPUI to panic.
    open_deferred_popovers: HashSet<ElementId>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self { text_view_state_stack: Vec::new(), open_deferred_popovers: HashSet::new() }
    }

    pub(crate) fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }

    /// Check if we are currently inside a deferred context (e.g., inside an open Popover).
    pub(crate) fn is_in_deferred_context(&self) -> bool {
        !self.open_deferred_popovers.is_empty()
    }

    /// Register a popover that uses deferred rendering as open.
    pub(crate) fn register_deferred_popover(&mut self, focus_handle: &FocusHandle) {
        self.open_deferred_popovers.insert(format!("{focus_handle:?}").into());
    }

    /// Unregister a popover when it closes.
    pub(crate) fn unregister_deferred_popover(&mut self, focus_handle: &FocusHandle) {
        let element_id: ElementId = format!("{focus_handle:?}").into();
        self.open_deferred_popovers.remove(&element_id);
    }
}
