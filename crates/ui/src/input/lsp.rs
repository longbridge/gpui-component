use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

use anyhow::Result;
use gpui::{App, Context, Task, Window};
use lsp_types::{request::Completion, CodeAction, CompletionContext, CompletionResponse};

use crate::input::InputState;

/// A trait for providing code completions based on the current input state and context.
pub trait CompletionProvider {
    /// Fetches completions based on the given byte offset.
    fn completions(
        &self,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<Vec<CompletionResponse>>>;

    fn resolve_completions(
        &self,
        _completion_indices: Vec<usize>,
        _completions: Rc<RefCell<Box<[Completion]>>>,
        _: &mut Context<InputState>,
    ) -> Task<Result<bool>> {
        Task::ready(Ok(false))
    }

    /// Determines if the completion should be triggered based on the given byte offset.
    fn is_completion_trigger(
        &self,
        offset: usize,
        new_text: &str,
        cx: &mut Context<InputState>,
    ) -> bool;
}

pub trait CodeActionProvider {
    /// The id for this CodeAction.
    fn id(&self) -> Arc<str>;

    /// Fetches code actions for the specified range.
    fn code_actions(
        &self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>>;
}
