use gpui::{App, Entity, Global};

use crate::text::{SelectionScope, TextViewState};

pub use gpui_base::GlobalState;

pub(crate) fn init(cx: &mut App) {
    // Preserve the legacy initialization point while `gpui_base::init` remains
    // after Root initialization for focus-trap ordering compatibility.
    GlobalState::init(cx);
    cx.set_global(UiGlobalState::new());
}

/// UI-only global state whose types cannot cross into `gpui-base`.
pub(crate) struct UiGlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
    selection_scope_stack: Vec<SelectionScope>,
}

impl Global for UiGlobalState {}

impl UiGlobalState {
    fn new() -> Self {
        Self {
            text_view_state_stack: Vec::new(),
            selection_scope_stack: Vec::new(),
        }
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

    pub(crate) fn push_selection_scope(&mut self, scope: SelectionScope) {
        self.selection_scope_stack.push(scope);
    }

    pub(crate) fn pop_selection_scope(&mut self) {
        self.selection_scope_stack.pop();
    }

    pub(crate) fn current_selection_scope(&self) -> SelectionScope {
        self.selection_scope_stack
            .last()
            .copied()
            .unwrap_or(SelectionScope::Base)
    }
}
