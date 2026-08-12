use anyhow::Result;
use gpui::{
    App, Context, Entity, EntityInputHandler, HighlightStyle, Hsla, MouseDownEvent, MouseMoveEvent,
    Pixels, SharedString, Task, Window, px,
};
use lsp_types::{
    CodeAction, ColorInformation, CompletionContext, CompletionItem, CompletionResponse, Hover,
    InlineCompletionContext, InlineCompletionItem, InlineCompletionResponse,
    InlineCompletionTriggerKind, SemanticTokens, SemanticTokensLegend, request::Completion,
};
use ropey::Rope;
use std::{cell::RefCell, ops::Range, rc::Rc, time::Duration};

use super::{InputState, RopeExt as _};
use crate::input::HighlightStyleResolver;

mod code_actions;
mod completions;
mod definitions;
mod document_colors;
mod hover;
mod overlay;
mod semantic_tokens;

pub use code_actions::*;
pub use completions::*;
pub use definitions::*;
pub use document_colors::*;
pub use hover::*;
pub use overlay::*;
pub use semantic_tokens::*;

pub type ShowDocumentHandler =
    Rc<dyn Fn(&lsp_types::ShowDocumentParams, &mut Window, &mut App) -> bool>;

pub struct Lsp {
    pub completion_provider: Option<Rc<dyn CompletionProvider>>,
    pub code_action_providers: Vec<Rc<dyn CodeActionProvider>>,
    pub hover_provider: Option<Rc<dyn HoverProvider>>,
    pub definition_provider: Option<Rc<dyn DefinitionProvider>>,
    pub document_color_provider: Option<Rc<dyn DocumentColorProvider>>,
    pub semantic_tokens_provider: Option<Rc<dyn DocumentRangeSemanticTokensProvider>>,
    pub show_document: Option<ShowDocumentHandler>,
    pub completion_menu: CompletionMenuOptions,
    pub(crate) document_colors: Vec<(lsp_types::Range, Hsla)>,
    pub(crate) semantic_tokens: Vec<(lsp_types::Range, SharedString)>,
    pub(crate) _hover_task: Task<Result<()>>,
    pub(crate) _document_color_task: Task<()>,
    pub(crate) _semantic_tokens_task: Task<()>,
}

impl Default for Lsp {
    fn default() -> Self {
        Self {
            completion_provider: None,
            code_action_providers: Vec::new(),
            hover_provider: None,
            definition_provider: None,
            document_color_provider: None,
            semantic_tokens_provider: None,
            show_document: None,
            completion_menu: CompletionMenuOptions::default(),
            document_colors: Vec::new(),
            semantic_tokens: Vec::new(),
            _hover_task: Task::ready(Ok(())),
            _document_color_task: Task::ready(()),
            _semantic_tokens_task: Task::ready(()),
        }
    }
}

impl Lsp {
    pub(crate) fn update(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        self.update_document_colors(text, window, cx);
        self.update_semantic_tokens(text, window, cx);
    }
    pub(crate) fn reset(&mut self) {
        self.document_colors.clear();
        self.semantic_tokens.clear();
        self._hover_task = Task::ready(Ok(()));
        self._document_color_task = Task::ready(());
        self._semantic_tokens_task = Task::ready(());
    }
}
