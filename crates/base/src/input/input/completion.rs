use std::ops::Range;

use gpui::{Action, Context, Window};
use lsp_types::CompletionItem;

use super::super::{CompletionMenuState, InputBaseState, InputMode, InputOverlayKind};

impl InputBaseState<InputMode> {
    /// Shows `items` in the completion menu, anchored at the caret.
    ///
    /// `trigger_start_offset` is where the text each item replaces begins and
    /// `query` is what the user has typed since. Items normally carry their own
    /// `text_edit`; otherwise the span from the start offset to the caret is
    /// replaced. An empty list closes the menu.
    pub fn present_completion_items(
        &mut self,
        trigger_start_offset: usize,
        query: impl Into<String>,
        items: Vec<CompletionItem>,
        cx: &mut Context<Self>,
    ) {
        if items.is_empty() && self.field_extras.is_none() {
            return;
        }
        let completion = &mut self
            .field_extras
            .get_or_insert_with(Default::default)
            .completion;
        completion.trigger_start_offset = Some(trigger_start_offset);
        completion.query = query.into();
        completion.items = items;
        completion.open = !completion.items.is_empty();
        completion.bump();
        cx.notify();
    }

    pub fn dismiss_completion_overlay(&mut self, cx: &mut Context<Self>) {
        self.hide_context_menu(cx);
    }

    pub fn insert_completion(
        &mut self,
        item: &CompletionItem,
        fallback_range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_completion_item(item, fallback_range, window, cx);
    }

    #[doc(hidden)]
    pub fn completion_menu_state(&self) -> &CompletionMenuState {
        static EMPTY: std::sync::LazyLock<CompletionMenuState> =
            std::sync::LazyLock::new(CompletionMenuState::default);
        self.field_extras
            .as_deref()
            .map(|extras| &extras.completion)
            .unwrap_or(&EMPTY)
    }

    #[doc(hidden)]
    pub fn route_overlay_action(
        &mut self,
        action: Box<dyn Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.handle_action_for_context_menu(action, window, cx)
    }

    pub fn set_overlay_action_handler(
        &mut self,
        handler: impl Fn(
            InputOverlayKind,
            Box<dyn Action>,
            &mut Window,
            &mut Context<InputBaseState<InputMode>>,
        ) -> bool
        + 'static,
    ) {
        self.overlay_action_handler = Some(std::rc::Rc::new(handler));
    }

    pub fn has_overlay_action_handler(&self) -> bool {
        self.overlay_action_handler.is_some()
    }

    pub(crate) fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        // Runs on every edit and caret move of every plain input; stay quiet
        // unless there is a menu to close.
        if let Some(extras) = self.field_extras.as_mut() {
            if extras.completion.open {
                extras.completion.open = false;
                cx.notify();
            }
        }
    }

    pub(crate) fn is_context_menu_open(&self) -> bool {
        self.field_extras
            .as_ref()
            .is_some_and(|extras| extras.completion.open)
    }

    pub(crate) fn handle_action_for_context_menu(
        &mut self,
        action: Box<dyn Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.is_context_menu_open() {
            return false;
        }
        let Some(handler) = self.overlay_action_handler.clone() else {
            return false;
        };
        let closes_overlay =
            crate::input::Enter::is_primary(&*action) || action.partial_eq(&crate::input::Escape);
        let handled = handler(InputOverlayKind::Completion, action, window, cx);
        if handled && closes_overlay {
            self.hide_context_menu(cx);
        }
        handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputModeKind;

    #[test]
    fn single_line_mode_keeps_its_unit_extras_contract() {
        fn accepts_unit_extras<M: InputModeKind<Extras = ()>>() {}
        accepts_unit_extras::<InputMode>();
    }
}
