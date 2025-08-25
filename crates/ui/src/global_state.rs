use gpui::{App, BorrowAppContext, Entity, Global};

use crate::text::TextViewState;

pub(crate) fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}

impl Global for GlobalState {}

pub(crate) struct GlobalState {
    text_view_state_stack: Vec<Entity<TextViewState>>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self {
            text_view_state_stack: Vec::new(),
        }
    }

    pub(crate) fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn with_text_view_state<R>(
        &mut self,
        state: Entity<TextViewState>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.text_view_state_stack.push(state);
        let result = f(self);
        self.text_view_state_stack.pop();
        result
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }
}
