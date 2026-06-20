use anyhow::Result;
use gpui::{App, Context, Hsla, MouseMoveEvent, SharedString, Task, Window};
use ropey::Rope;
use std::rc::Rc;

use crate::input::{InputState, RopeExt, popovers::ContextMenu};

mod code_actions;
mod completions;
mod definitions;
mod document_colors;
mod hover;
mod semantic_tokens;

pub use code_actions::*;
pub use completions::*;
pub use definitions::*;
pub use document_colors::*;
pub use hover::*;
pub use semantic_tokens::*;

/// LSP ServerCapabilities
///
/// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#serverCapabilities
pub struct Lsp {
    /// The completion provider.
    pub completion_provider: Option<Rc<dyn CompletionProvider>>,
    /// The code action providers.
    pub code_action_providers: Vec<Rc<dyn CodeActionProvider>>,
    /// The hover provider.
    pub hover_provider: Option<Rc<dyn HoverProvider>>,
    /// The definition provider.
    pub definition_provider: Option<Rc<dyn DefinitionProvider>>,
    /// The document color provider.
    pub document_color_provider: Option<Rc<dyn DocumentColorProvider>>,
    /// The range semantic tokens provider.
    pub semantic_tokens_provider: Option<Rc<dyn DocumentRangeSemanticTokensProvider>>,

    document_colors: Vec<(lsp_types::Range, Hsla)>,
    /// Cached semantic tokens as absolute position ranges + theme token-type
    /// names. Color is resolved from the name at paint time so theme switches
    /// take effect without a refetch.
    semantic_tokens: Vec<(lsp_types::Range, SharedString)>,
    _hover_task: Task<Result<()>>,
    _document_color_task: Task<()>,
    _semantic_tokens_task: Task<()>,
}

impl Default for Lsp {
    fn default() -> Self {
        Self {
            completion_provider: None,
            code_action_providers: vec![],
            hover_provider: None,
            definition_provider: None,
            document_color_provider: None,
            semantic_tokens_provider: None,
            document_colors: vec![],
            semantic_tokens: vec![],
            _hover_task: Task::ready(Ok(())),
            _document_color_task: Task::ready(()),
            _semantic_tokens_task: Task::ready(()),
        }
    }
}

impl Lsp {
    /// Update the LSP when the text changes.
    pub(crate) fn update(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        self.update_document_colors(text, window, cx);
        self.update_semantic_tokens(text, window, cx);
    }

    /// Reset all LSP states.
    pub(crate) fn reset(&mut self) {
        self.document_colors.clear();
        self.semantic_tokens.clear();
        self._hover_task = Task::ready(Ok(()));
        self._document_color_task = Task::ready(());
        self._semantic_tokens_task = Task::ready(());
    }
}

impl InputState {
    /// Set the LSP completion provider.
    pub fn set_lsp_completion_provider(
        &mut self,
        provider: Option<Rc<dyn CompletionProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.completion_provider = provider;
        self._pending_update = true;
        cx.notify();
    }

    /// Set the LSP hover provider.
    pub fn set_lsp_hover_provider(
        &mut self,
        provider: Option<Rc<dyn HoverProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.hover_provider = provider;
        self._pending_update = true;
        cx.notify();
    }

    /// Set the LSP definition provider.
    pub fn set_lsp_definition_provider(
        &mut self,
        provider: Option<Rc<dyn DefinitionProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.definition_provider = provider;
        self._pending_update = true;
        cx.notify();
    }

    /// Set the LSP document color provider.
    pub fn set_lsp_document_color_provider(
        &mut self,
        provider: Option<Rc<dyn DocumentColorProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.document_color_provider = provider;
        self._pending_update = true;
        cx.notify();
    }

    /// Set the LSP semantic tokens provider.
    pub fn set_lsp_semantic_tokens_provider(
        &mut self,
        provider: Option<Rc<dyn DocumentRangeSemanticTokensProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.semantic_tokens_provider = provider;
        self._pending_update = true;
        cx.notify();
    }

    /// Replace all LSP code action providers.
    pub fn set_lsp_code_action_providers(
        &mut self,
        providers: Vec<Rc<dyn CodeActionProvider>>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.code_action_providers = providers;
        self._pending_update = true;
        cx.notify();
    }

    /// Clear all LSP code action providers.
    pub fn clear_lsp_code_action_providers(&mut self, cx: &mut Context<Self>) {
        self.lsp.code_action_providers.clear();
        self._pending_update = true;
        cx.notify();
    }

    /// Append an LSP code action provider.
    pub fn push_lsp_code_action_provider(
        &mut self,
        provider: Rc<dyn CodeActionProvider>,
        cx: &mut Context<Self>,
    ) {
        self.lsp.code_action_providers.push(provider);
        self._pending_update = true;
        cx.notify();
    }

    pub(crate) fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu_content = None;
        self._context_menu_task = Task::ready(Ok(()));
        cx.notify();
    }

    pub(crate) fn is_context_menu_open(&self, cx: &App) -> bool {
        let Some(menu) = self.context_menu_content.as_ref() else {
            return false;
        };

        menu.is_open(cx)
    }

    /// Handles an action for the completion menu, if it exists.
    ///
    /// Return true if the action was handled, otherwise false.
    pub fn handle_action_for_context_menu(
        &mut self,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(menu) = self.context_menu_content.as_ref() else {
            return false;
        };

        let mut handled = false;

        match menu {
            ContextMenu::Completion(menu) => {
                _ = menu.update(cx, |menu, cx| {
                    handled = menu.handle_action(action, window, cx)
                });
            }
            ContextMenu::CodeAction(menu) => {
                _ = menu.update(cx, |menu, cx| {
                    handled = menu.handle_action(action, window, cx)
                });
            }
        };

        handled
    }

    /// Apply a list of [`lsp_types::TextEdit`] to mutate the text.
    pub fn apply_lsp_edits(
        &mut self,
        text_edits: &Vec<lsp_types::TextEdit>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for edit in text_edits {
            let start = self.text.position_to_offset(&edit.range.start);
            let end = self.text.position_to_offset(&edit.range.end);

            let range_utf16 = self.range_to_utf16(&(start..end));
            self.replace_text_in_range_silent(Some(range_utf16), &edit.new_text, window, cx);
        }
    }

    pub(super) fn handle_mouse_move(
        &mut self,
        offset: usize,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        if event.modifiers.secondary() {
            self.handle_hover_definition(offset, window, cx);
        } else {
            self.hover_definition.clear();
            self.handle_hover_popover(offset, window, cx);
        }
        cx.notify();
    }

    pub(crate) fn clear_hover_state(&mut self, cx: &mut Context<InputState>) {
        self.hover_definition.clear();
        self.hover_popover = None;
        self.lsp._hover_task = Task::ready(Ok(()));
        cx.notify();
    }
}
