use anyhow::Result;
use gpui::{Context, EntityInputHandler, Task, Window};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionResponse, InlineCompletionContext,
    InlineCompletionItem, InlineCompletionResponse, InlineCompletionTriggerKind,
    request::Completion,
};
use ropey::Rope;
use std::{cell::RefCell, ops::Range, rc::Rc, time::Duration};

use crate::input::{
    InputState, RopeExt,
    popovers::{CompletionMenu, ContextMenu},
};

/// Default debounce duration for inline completions.
const DEFAULT_INLINE_COMPLETION_DEBOUNCE: Duration = Duration::from_millis(300);

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

        if new_text == " " {
            self._context_menu_task = Task::ready(Ok(()));
            self.hide_context_menu(cx);
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
        let manual_trigger = new_text.is_empty() && range.start == range.end;

        if new_text.is_empty() && !self.is_context_menu_open(cx) && !manual_trigger {
            return;
        }

        if let Some(menu) = self.context_menu.as_ref() {
            if let Some(c_menu) = menu.as_completion_menu() {
                if let Some(menu_start) = c_menu.read(cx).trigger_start_offset {
                    if new_offset <= menu_start && !manual_trigger {
                        self._context_menu_task = Task::ready(Ok(()));
                        self.hide_context_menu(cx);
                        return;
                    }
                }
            }
        }

        if !provider.is_completion_trigger(start, new_text, cx) {
            return;
        }

        if !matches!(self.context_menu, Some(ContextMenu::Completion(_))) {
            let menu = CompletionMenu::new(cx.entity(), self.autocompletion_menu_width, window, cx);
            self.context_menu = Some(ContextMenu::Completion(menu));
        }

        let Some(menu_entity) = self
            .context_menu
            .as_ref()
            .and_then(|m| m.as_completion_menu())
        else {
            return;
        };
        let menu_entity = menu_entity.clone();

        let mut start_offset = menu_entity
            .read(cx)
            .trigger_start_offset
            .unwrap_or(range.start);

        if manual_trigger {
            let mut word_start = range.start;
            let text = self.text.clone();
            for c in text.chars_at(range.start).reversed() {
                if c.is_alphanumeric() || c.is_ascii_punctuation() {
                    word_start -= c.len_utf8();
                } else {
                    break;
                }
            }
            start_offset = word_start;
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

        if query.is_empty() && !manual_trigger {
            self._context_menu_task = Task::ready(Ok(()));
            self.hide_context_menu(cx);
            return;
        }
        _ = menu_entity.update(cx, |menu: &mut CompletionMenu, _| {
            menu.update_query(start_offset, query.clone());
        });

        let completion_context = CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(query),
        };

        let provider_responses =
            provider.completions(&self.text, new_offset, completion_context, window, cx);

        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut completions: Vec<CompletionItem> = vec![];
            if let Some(resp) = provider_responses.await.ok() {
                match resp {
                    CompletionResponse::Array(items) => completions.extend(items),
                    CompletionResponse::List(list) => completions.extend(list.items),
                }
            }

            editor
                .update_in(cx, |editor, window, cx| {
                    if !editor.focus_handle.is_focused(window) && !manual_trigger {
                        return;
                    }

                    if completions.is_empty() {
                        _ = menu_entity.update(cx, |menu, cx| {
                            if manual_trigger {
                                menu.show(new_offset, completions, window, cx);
                            } else {
                                menu.hide(cx);
                            }
                        });
                    } else {
                        _ = menu_entity.update(cx, |menu, cx| {
                            menu.show(new_offset, completions, window, cx);
                        });
                    }
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

pub struct KeywordCompletionProvider {
    keywords: Vec<String>,
}

impl KeywordCompletionProvider {
    pub fn new(keywords: Vec<String>) -> Self {
        Self { keywords }
    }
}

impl CompletionProvider for KeywordCompletionProvider {
    fn is_completion_trigger(
        &self,
        _offset: usize,
        _new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger automatically when typing
        true
    }

    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let text = text.clone();
        let keywords = self.keywords.clone();

        cx.background_executor().spawn(async move {
            let prefix_chars: String = text
                .chars_at(offset)
                .reversed()
                .take_while(|c| c.is_alphanumeric() || c.is_ascii_punctuation())
                .collect();

            let prefix = prefix_chars
                .chars()
                .rev()
                .collect::<String>()
                .to_lowercase();

            let start = offset - prefix.len();
            let prefix = text.slice(start..offset).to_string().to_lowercase();

            let lsp_range = lsp_types::Range {
                start: text.offset_to_position(start),
                end: text.offset_to_position(offset),
            };

            let mut matches: Vec<(String, i32)> = if prefix.is_empty() {
                keywords.into_iter().map(|kw| (kw, 1)).collect()
            } else {
                keywords
                    .into_iter()
                    .filter_map(|kw| {
                        let kw_lower = kw.to_lowercase();
                        if kw_lower == prefix {
                            return Some((kw, 100));
                        }
                        if kw_lower.starts_with(&prefix) {
                            return Some((kw, 50));
                        }
                        if is_subsequence(&prefix, &kw_lower) {
                            return Some((kw, 10));
                        }
                        None
                    })
                    .collect()
            };
            // High score first, then by case
            matches.sort_by(|(kw_a, score_a), (kw_b, score_b)| {
                score_b
                    .cmp(score_a)
                    .then_with(|| kw_a.to_lowercase().cmp(&kw_b.to_lowercase()))
            });

            let items = matches
                .into_iter()
                .map(|(kw, _score)| CompletionItem {
                    label: kw.clone(),
                    kind: Some(lsp_types::CompletionItemKind::KEYWORD),
                    text_edit: Some(lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
                        range: lsp_range,
                        new_text: kw,
                    })),
                    ..Default::default()
                })
                .collect();

            Ok(CompletionResponse::Array(items))
        })
    }

    fn inline_completion(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<InlineCompletionResponse>> {
        let text = text.clone();
        let keywords = self.keywords.clone();

        cx.background_executor().spawn(async move {
            let prefix_chars: String = text
                .chars_at(offset)
                .reversed()
                .take_while(|c| c.is_alphanumeric() || c.is_ascii_punctuation())
                .collect();

            let prefix = prefix_chars
                .chars()
                .rev()
                .collect::<String>()
                .to_lowercase();

            if prefix.is_empty() {
                return Ok(InlineCompletionResponse::Array(vec![]));
            }

            let item = keywords
                .into_iter()
                .find(|kw| kw.to_lowercase().starts_with(&prefix) && kw.to_lowercase() != prefix)
                .map(|kw| InlineCompletionItem {
                    insert_text: kw[prefix.len()..].to_string(),
                    filter_text: None,
                    range: None,
                    command: None,
                    insert_text_format: None,
                });

            Ok(InlineCompletionResponse::Array(item.into_iter().collect()))
        })
    }
}

fn is_subsequence(pattern: &str, text: &str) -> bool {
    let mut pat_iter = pattern.chars();
    let mut current_pat_char = pat_iter.next();
    for c in text.chars() {
        match current_pat_char {
            Some(p) if p == c => current_pat_char = pat_iter.next(),
            Some(_) => continue,
            None => return true,
        }
    }
    current_pat_char.is_none()
}
