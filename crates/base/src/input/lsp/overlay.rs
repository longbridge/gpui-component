use super::*;

#[derive(Clone, Debug, Default)]
pub struct CompletionSession {
    pub open: bool,
    pub trigger_start_offset: Option<usize>,
    pub query: String,
    pub items: Vec<CompletionItem>,
}

#[derive(Clone, Debug, Default)]
pub struct CodeActionSession {
    pub open: bool,
    pub items: Vec<CodeActionItem>,
}

#[derive(Clone, Debug)]
pub struct HoverSession {
    pub symbol_range: Range<usize>,
    pub hover: Hover,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOverlayKind {
    Completion,
    CodeAction,
}

impl InputState {
    pub fn present_completion_items(
        &mut self,
        trigger_start_offset: usize,
        query: impl Into<String>,
        items: Vec<CompletionItem>,
        cx: &mut Context<Self>,
    ) {
        self.completion_session.trigger_start_offset = Some(trigger_start_offset);
        self.completion_session.query = query.into();
        self.completion_session.items = items;
        self.completion_session.open = !self.completion_session.items.is_empty();
        cx.notify();
    }

    pub fn present_code_actions(&mut self, items: Vec<CodeActionItem>, cx: &mut Context<Self>) {
        self.code_action_session.items = items;
        self.code_action_session.open = !self.code_action_session.items.is_empty();
        cx.notify();
    }

    pub fn present_hover(
        &mut self,
        symbol_range: Range<usize>,
        hover: Hover,
        cx: &mut Context<Self>,
    ) {
        self.hover_session = Some(HoverSession {
            symbol_range,
            hover,
        });
        cx.notify();
    }

    pub fn present_diagnostic(
        &mut self,
        diagnostic: crate::input::DiagnosticEntry,
        cx: &mut Context<Self>,
    ) {
        self.diagnostic_overlay = Some(Rc::new(diagnostic));
        cx.notify();
    }

    pub fn clear_diagnostic_overlay(&mut self, cx: &mut Context<Self>) {
        if self.diagnostic_overlay.take().is_some() {
            cx.notify();
        }
    }

    pub fn route_overlay_action(
        &mut self,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.handle_action_for_context_menu(action, window, cx)
    }

    pub fn set_overlay_action_handler(
        &mut self,
        handler: impl Fn(
            InputOverlayKind,
            Box<dyn gpui::Action>,
            &mut Window,
            &mut Context<InputState>,
        ) -> bool
        + 'static,
    ) {
        self.overlay_action_handler = Some(Rc::new(handler));
    }

    pub fn has_overlay_action_handler(&self) -> bool {
        self.overlay_action_handler.is_some()
    }

    pub fn dismiss_completion_overlay(&mut self, cx: &mut Context<Self>) {
        if self.completion_session.open {
            self.completion_session.open = false;
            cx.notify();
        }
    }

    pub fn dismiss_code_action_overlay(&mut self, cx: &mut Context<Self>) {
        if self.code_action_session.open {
            self.code_action_session.open = false;
            cx.notify();
        }
    }

    pub fn completion_session(&self) -> &CompletionSession {
        &self.completion_session
    }

    pub fn code_action_session(&self) -> &CodeActionSession {
        &self.code_action_session
    }

    pub fn hover_session(&self) -> Option<&HoverSession> {
        self.hover_session.as_ref()
    }

    pub fn dismiss_lsp_overlays(&mut self, cx: &mut Context<Self>) {
        self.hide_context_menu(cx);
        self.clear_hover_state(cx);
    }
}
