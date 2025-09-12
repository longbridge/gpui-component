use std::{cell::RefCell, ops::Range, rc::Rc, sync::Arc};

use anyhow::Result;
use gpui::{App, Context, EntityInputHandler, Task, Window};
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
    pub(crate) fn is_completion_menu_open(&self, cx: &App) -> bool {
        let Some(menu) = self.completion_menu.as_ref() else {
            return false;
        };

        menu.read(cx).is_open()
    }

    /// Handles an action for the completion menu, if it exists.
    ///
    /// Return true if the action was handled, otherwise false.
    pub fn handle_action_for_completion_menu(
        &mut self,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(menu) = self.completion_menu.as_ref() else {
            return false;
        };

        let mut handled = false;
        _ = menu.update(cx, |menu, cx| {
            handled = menu.handle_action(action, window, cx)
        });

        handled
    }

    pub fn handle_completion_trigger(
        &mut self,
        range: &Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.completion_inserting {
            return;
        }

        let Some(provider) = self.mode.completion_provider().cloned() else {
            return;
        };

        let start = range.end;
        let new_offset = self.cursor();

        if !provider.is_completion_trigger(start, new_text, cx) {
            return;
        }

        // To create or get the existing completion menu.
        let completion_menu = match self.completion_menu.as_ref() {
            Some(menu) => menu.clone(),
            None => {
                let menu = CompletionMenu::new(cx.entity(), window, cx);
                self.completion_menu = Some(menu.clone());
                menu
            }
        };

        let start_offset = completion_menu
            .read(cx)
            .trigger_start_offset
            .unwrap_or(start);
        if new_offset < start_offset {
            return;
        }

        let query = self
            .text_for_range(
                self.range_to_utf16(&(start_offset..new_offset)),
                &mut None,
                window,
                cx,
            )
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        _ = completion_menu.update(cx, |menu, _| {
            menu.update_query(start_offset, query.clone());
        });

        let completion_context = CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(query),
        };

        let provider_responses = provider.completions(start_offset, completion_context, window, cx);

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
                _ = completion_menu.update(cx, |menu, cx| {
                    menu.hide(cx);
                    cx.notify();
                });

                return Ok(());
            }

            editor
                .update_in(cx, |editor, window, cx| {
                    if !editor.focus_handle.is_focused(window) {
                        return;
                    }

                    _ = completion_menu.update(cx, |menu, cx| {
                        menu.show(new_offset, completions, window, cx);
                    });

                    cx.notify();
                })
                .ok();

            Ok(())
        });
    }
}
