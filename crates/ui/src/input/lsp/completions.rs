use anyhow::Result;
use gpui::{Context, EntityInputHandler, Task, Window};
use lsp_types::{
    request::Completion, CompletionContext, CompletionItem, CompletionResponse,
    InlineCompletionContext, InlineCompletionItem, InlineCompletionResponse,
    InlineCompletionTriggerKind,
};
use ropey::Rope;
use std::{cell::RefCell, ops::Range, rc::Rc, time::Duration};

use crate::input::{
    popovers::{CompletionMenu, ContextMenu},
    InputState,
};

/// Default debounce duration for inline completions.
const DEFAULT_INLINE_COMPLETION_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionMenuAction {
    Ignore,
    Hide,
    Refresh(lsp_types::CompletionTriggerKind),
}

fn completion_menu_action(
    has_existing_menu: bool,
    is_trigger: bool,
    full_text: &str,
    new_offset: usize,
    start_offset: usize,
) -> CompletionMenuAction {
    if !has_existing_menu && !is_trigger {
        return CompletionMenuAction::Ignore;
    }

    if has_existing_menu && (full_text.trim().is_empty() || new_offset < start_offset) {
        return CompletionMenuAction::Hide;
    }

    if is_trigger {
        CompletionMenuAction::Refresh(lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER)
    } else {
        CompletionMenuAction::Refresh(lsp_types::CompletionTriggerKind::INVOKED)
    }
}

/// A trait for providing code completions based on the current input state and context.
pub trait CompletionProvider {
    /// Fetches completions based on the given byte offset.
    ///
    /// - The `offset` is in bytes of current cursor.
    ///
    /// textDocument/completion
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>>;

    /// Fetches an inline completion suggestion for the given position.
    ///
    /// This is called after a debounce period when the user stops typing.
    /// The provider can analyze the text and cursor position to determine
    /// what inline completion suggestion to show.
    ///
    ///
    /// # Arguments
    /// * `rope` - The current text content
    /// * `offset` - The cursor position in bytes
    ///
    /// textDocument/inlineCompletion
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#textDocument_inlineCompletion
    fn inline_completion(
        &self,
        _rope: &Rope,
        _offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<InlineCompletionResponse>> {
        Task::ready(Ok(InlineCompletionResponse::Array(vec![])))
    }

    /// Returns the debounce duration for inline completions.
    ///
    /// Default: 300ms
    #[inline]
    fn inline_completion_debounce(&self) -> Duration {
        DEFAULT_INLINE_COMPLETION_DEBOUNCE
    }

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

pub(crate) struct InlineCompletion {
    /// Completion item to display as an inline completion suggestion
    pub(crate) item: Option<InlineCompletionItem>,
    /// Task for debouncing inline completion requests
    pub(crate) task: Task<Result<InlineCompletionResponse>>,
}

impl Default for InlineCompletion {
    fn default() -> Self {
        Self {
            item: None,
            task: Task::ready(Ok(InlineCompletionResponse::Array(vec![]))),
        }
    }
}

impl InputState {
    pub(crate) fn handle_completion_trigger(
        &mut self,
        range: &Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.completion_inserting {
            return;
        }

        let Some(provider) = self.lsp.completion_provider.clone() else {
            return;
        };

        // Always schedule inline completion (debounced).
        // It will check if menu is open before showing the suggestion.
        self.schedule_inline_completion(window, cx);

        let start = range.end;
        let new_offset = self.cursor();
        let existing_menu = match self.context_menu.as_ref() {
            Some(ContextMenu::Completion(menu)) => Some(menu),
            _ => None,
        };
        let is_trigger = provider.is_completion_trigger(start, new_text, cx);
        let start_offset = existing_menu
            .as_ref()
            .and_then(|menu| menu.read(cx).trigger_start_offset)
            .unwrap_or(start);
        let action = completion_menu_action(
            existing_menu.is_some(),
            is_trigger,
            &self.text.to_string(),
            new_offset,
            start_offset,
        );

        match action {
            CompletionMenuAction::Ignore => return,
            CompletionMenuAction::Hide => {
                if let Some(menu) = existing_menu {
                    _ = menu.update(cx, |menu, cx| {
                        menu.hide(cx);
                    });
                }
                return;
            }
            CompletionMenuAction::Refresh(_) => {}
        }

        // To create or get the existing completion menu.
        let menu = match existing_menu {
            Some(menu) => menu.clone(),
            None => {
                let menu = CompletionMenu::new(cx.entity(), window, cx);
                self.context_menu = Some(ContextMenu::Completion(menu.clone()));
                menu
            }
        };

        let query = self
            .text_for_range(
                self.range_to_utf16(&(start_offset..new_offset)),
                &mut None,
                window,
                cx,
            )
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        _ = menu.update(cx, |menu, _| {
            menu.update_query(start_offset, query.clone());
        });

        let completion_context = CompletionContext {
            trigger_kind: match action {
                CompletionMenuAction::Refresh(trigger_kind) => trigger_kind,
                CompletionMenuAction::Ignore | CompletionMenuAction::Hide => {
                    lsp_types::CompletionTriggerKind::INVOKED
                }
            },
            trigger_character: Some(query),
        };

        let provider_responses =
            provider.completions(&self.text, new_offset, completion_context, window, cx);
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut completions: Vec<CompletionItem> = vec![];
            if let Some(provider_responses) = provider_responses.await.ok() {
                match provider_responses {
                    CompletionResponse::Array(items) => completions.extend(items),
                    CompletionResponse::List(list) => completions.extend(list.items),
                }
            }

            if completions.is_empty() {
                _ = menu.update(cx, |menu, cx| {
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

                    _ = menu.update(cx, |menu, cx| {
                        menu.show(new_offset, completions, window, cx);
                    });

                    cx.notify();
                })
                .ok();

            Ok(())
        });
    }

    /// Schedule an inline completion request after debouncing.
    pub(crate) fn schedule_inline_completion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Clear any existing inline completion on text change
        self.clear_inline_completion(cx);

        let Some(provider) = self.lsp.completion_provider.clone() else {
            return;
        };

        let offset = self.cursor();
        let text = self.text.clone();
        let debounce = provider.inline_completion_debounce();

        self.inline_completion.task = cx.spawn_in(window, async move |editor, cx| {
            // Debounce: wait before fetching to avoid unnecessary requests while typing
            smol::Timer::after(debounce).await;

            // Now fetch the inline completion after the debounce period
            let task = editor.update_in(cx, |editor, window, cx| {
                // Check if cursor has moved during debounce
                if editor.cursor() != offset {
                    return None;
                }

                // Don't fetch if completion menu is open
                if editor.is_context_menu_open(cx) {
                    return None;
                }

                let trigger = InlineCompletionContext {
                    trigger_kind: InlineCompletionTriggerKind::Automatic,
                    selected_completion_info: None,
                };

                Some(provider.inline_completion(&text, offset, trigger, window, cx))
            })?;

            let Some(task) = task else {
                return Ok(InlineCompletionResponse::Array(vec![]));
            };

            let response = task.await?;

            editor.update_in(cx, |editor, _window, cx| {
                // Only apply if cursor still hasn't moved
                if editor.cursor() != offset {
                    return;
                }

                // Don't show if completion menu opened while we were fetching
                if editor.is_context_menu_open(cx) {
                    return;
                }

                if let Some(item) = match response.clone() {
                    InlineCompletionResponse::Array(items) => items.into_iter().next(),
                    InlineCompletionResponse::List(comp_list) => comp_list.items.into_iter().next(),
                } {
                    editor.inline_completion.item = Some(item);
                    cx.notify();
                }
            })?;

            Ok(response)
        });
    }

    /// Check if an inline completion suggestion is currently displayed.
    #[inline]
    pub(crate) fn has_inline_completion(&self) -> bool {
        self.inline_completion.item.is_some()
    }

    /// Clear the inline completion suggestion.
    pub(crate) fn clear_inline_completion(&mut self, cx: &mut Context<Self>) {
        self.inline_completion = InlineCompletion::default();
        cx.notify();
    }

    /// Accept the inline completion, inserting it at the cursor position.
    /// Returns true if a completion was accepted, false if there was none.
    pub(crate) fn accept_inline_completion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(completion_item) = self.inline_completion.item.take() else {
            return false;
        };

        let cursor = self.cursor();
        let range_utf16 = self.range_to_utf16(&(cursor..cursor));
        let completion_text = completion_item.insert_text;
        self.replace_text_in_range_silent(Some(range_utf16), &completion_text, window, cx);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{completion_menu_action, CompletionMenuAction};
    use lsp_types::CompletionTriggerKind;

    #[test]
    fn ignores_non_trigger_without_existing_menu() {
        assert_eq!(
            completion_menu_action(false, false, "name", 4, 0),
            CompletionMenuAction::Ignore
        );
    }

    #[test]
    fn hides_existing_menu_when_text_becomes_empty() {
        assert_eq!(
            completion_menu_action(true, false, "", 0, 0),
            CompletionMenuAction::Hide
        );
        assert_eq!(
            completion_menu_action(true, false, "   ", 0, 0),
            CompletionMenuAction::Hide
        );
    }

    #[test]
    fn hides_existing_menu_when_cursor_moves_before_trigger_start() {
        assert_eq!(
            completion_menu_action(true, false, "na", 0, 1),
            CompletionMenuAction::Hide
        );
    }

    #[test]
    fn refreshes_existing_menu_on_delete_when_text_still_has_context() {
        assert_eq!(
            completion_menu_action(true, false, "n", 1, 0),
            CompletionMenuAction::Refresh(CompletionTriggerKind::INVOKED)
        );
    }

    #[test]
    fn refreshes_with_trigger_character_for_normal_typing() {
        assert_eq!(
            completion_menu_action(false, true, "na", 2, 2),
            CompletionMenuAction::Refresh(CompletionTriggerKind::TRIGGER_CHARACTER)
        );
    }
}
