use gpui::{App, ElementId, Entity, FocusHandle, Global, OwnedMenu};
use std::collections::HashSet;

use crate::text::{SelectionScope, TextViewState};

pub(crate) fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}

impl Global for GlobalState {}

pub struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
    /// Set of open popover IDs that use deferred rendering.
    /// When this set is not empty, we are inside at least one deferred context.
    /// This is used to prevent double-deferred elements which would cause GPUI to panic.
    open_deferred_popovers: HashSet<ElementId>,
    /// Application menus storage
    app_menus: Vec<OwnedMenu>,
    /// When true, the window-level text selection must not start on the
    /// current mouse down. Set by components that own their own mouse-down
    /// interaction (e.g. `Input`, `Button`); reset by the selection
    /// controller in the capture phase of every left mouse down.
    pub(crate) suppress_text_selection: bool,
    /// Stack of active text-selection scopes, pushed/popped by the
    /// `SelectionScopeMarker` element that wraps each Dialog/Sheet content
    /// subtree during paint. Empty means the base window layer. A selectable
    /// `TextView` reads the top of this stack when it registers, so window
    /// selection can be confined to the active modal.
    selection_scope_stack: Vec<SelectionScope>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self {
            text_view_state_stack: Vec::new(),
            open_deferred_popovers: HashSet::new(),
            app_menus: Vec::new(),
            suppress_text_selection: false,
            selection_scope_stack: Vec::new(),
        }
    }

    /// Suppress the window-level text selection for the current mouse down.
    ///
    /// Call this from a mouse-down handler (bubble phase) of a component that
    /// owns its own press/drag interaction, so that pressing it does not start
    /// a window text selection. The flag is reset on the next mouse down.
    pub fn suppress_text_selection(cx: &mut App) {
        Self::global_mut(cx).suppress_text_selection = true;
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }

    /// Push a selection scope while painting a Dialog/Sheet content subtree.
    pub(crate) fn push_selection_scope(&mut self, scope: SelectionScope) {
        self.selection_scope_stack.push(scope);
    }

    /// Pop the selection scope after painting a Dialog/Sheet content subtree.
    pub(crate) fn pop_selection_scope(&mut self) {
        self.selection_scope_stack.pop();
    }

    /// The selection scope currently being painted. `Base` when not inside any
    /// Dialog/Sheet content subtree.
    pub(crate) fn current_selection_scope(&self) -> SelectionScope {
        self.selection_scope_stack
            .last()
            .copied()
            .unwrap_or(SelectionScope::Base)
    }

    /// Check if we are currently inside a deferred context (e.g., inside an open Popover).
    pub(crate) fn is_in_deferred_context(&self) -> bool {
        !self.open_deferred_popovers.is_empty()
    }

    /// Register a popover that uses deferred rendering as open.
    pub(crate) fn register_deferred_popover(&mut self, focus_handle: &FocusHandle) {
        self.open_deferred_popovers
            .insert(format!("{focus_handle:?}").into());
    }

    /// Unregister a popover when it closes.
    pub(crate) fn unregister_deferred_popover(&mut self, focus_handle: &FocusHandle) {
        let element_id: ElementId = format!("{focus_handle:?}").into();
        self.open_deferred_popovers.remove(&element_id);
    }

    /// Get the application menus
    pub fn app_menus(&self) -> &[OwnedMenu] {
        &self.app_menus
    }

    /// Set the application menus
    pub fn set_app_menus(&mut self, menus: Vec<OwnedMenu>) {
        self.app_menus = menus;
    }
}
