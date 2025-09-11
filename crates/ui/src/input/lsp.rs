use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

use anyhow::Result;
use gpui::{App, Context, Task, Window};
use lsp_types::{
    request::Completion, CodeAction, CompletionContext, CompletionItem, CompletionResponse,
};

use crate::input::{code_context_menu::CompletionMenu, InputState};

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
    ///
    /// This is called on the main thread.
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

impl InputState {
    pub fn handle_completion_trigger(
        &mut self,
        range: &Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self.mode.completion_provider().cloned() else {
            return;
        };

        let offset = range.end;
        let new_offset = offset.saturating_sub(new_text.len());

        if !provider.is_completion_trigger(offset, new_text, cx) {
            return;
        }

        let completion_context = CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(new_text.to_string()),
        };

        let provider_responses = provider.completions(offset, completion_context, window, cx);

        self._completion_task = cx.spawn_in(window, async move |editor, cx| {
            let mut completions: Vec<CompletionItem> = vec![];
            if let Some(provider_responses) = provider_responses.await.ok() {
                for resp in provider_responses {
                    match resp {
                        CompletionResponse::Array(items) => completions.extend(items),
                        CompletionResponse::List(list) => completions.extend(list.items),
                    }
                }
            }

            if completions.is_empty() {
                _ = editor.update_in(cx, |editor, _, cx| {
                    editor.completion_menu = None;
                    cx.notify();
                });

                return Ok(());
            }

            editor
                .update_in(cx, |editor, window, cx| {
                    if !editor.focus_handle.is_focused(window) {
                        return;
                    }

                    if let Some(menu) = editor.completion_menu.as_mut() {
                        menu.update(cx, |menu, cx| {
                            menu.show(new_offset, completions, cx);
                        })
                    } else {
                        editor.completion_menu = Some(CompletionMenu::new(
                            cx.entity(),
                            new_offset,
                            completions,
                            true,
                            window,
                            cx,
                        ));
                    };
                    cx.notify();
                })
                .ok();

            Ok(())
        });
    }
}
