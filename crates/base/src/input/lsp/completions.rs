use super::*;

const DEFAULT_INLINE_COMPLETION_DEBOUNCE: Duration = Duration::from_millis(300);

pub trait CompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>>;

    fn inline_completion(
        &self,
        _text: &Rope,
        _offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<InlineCompletionResponse>> {
        Task::ready(Ok(InlineCompletionResponse::Array(vec![])))
    }

    fn inline_completion_debounce(&self) -> Duration {
        DEFAULT_INLINE_COMPLETION_DEBOUNCE
    }

    fn resolve_completions(
        &self,
        _indices: Vec<usize>,
        _completions: Rc<RefCell<Box<[Completion]>>>,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<bool>> {
        Task::ready(Ok(false))
    }

    fn is_completion_trigger(
        &self,
        offset: usize,
        new_text: &str,
        cx: &mut Context<InputState>,
    ) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub struct CompletionMenuOptions {
    pub max_width: Pixels,
}

impl Default for CompletionMenuOptions {
    fn default() -> Self {
        Self {
            max_width: px(320.),
        }
    }
}

pub struct InlineCompletion {
    pub(crate) item: Option<InlineCompletionItem>,
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
    pub fn insert_completion(
        &mut self,
        item: &CompletionItem,
        fallback_range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut range = fallback_range;
        let mut new_text = item.label.clone();
        if let Some(edit) = item.text_edit.as_ref() {
            match edit {
                lsp_types::CompletionTextEdit::Edit(edit) => {
                    new_text.clone_from(&edit.new_text);
                    range = self.text.position_to_offset(&edit.range.start)
                        ..self.text.position_to_offset(&edit.range.end);
                }
                lsp_types::CompletionTextEdit::InsertAndReplace(edit) => {
                    new_text.clone_from(&edit.new_text);
                    range = self.text.position_to_offset(&edit.replace.start)
                        ..self.text.position_to_offset(&edit.replace.end);
                }
            }
        } else if let Some(insert_text) = item.insert_text.as_ref() {
            new_text.clone_from(insert_text);
            range = range.end..range.end;
        }
        self.completion_inserting = true;
        let range = self.range_to_utf16(&range);
        self.replace_text_in_range_silent(Some(range), &new_text, window, cx);
        self.completion_inserting = false;
        self.focus(window, cx);
    }
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
        self.schedule_inline_completion(window, cx);
        let start = range.end;
        let offset = self.cursor();
        if !provider.is_completion_trigger(start, new_text, cx) {
            return;
        }
        let trigger_start = self
            .completion_session
            .trigger_start_offset
            .unwrap_or(start);
        if offset < trigger_start {
            return;
        }
        let query = self
            .text_for_range(
                self.range_to_utf16(&(trigger_start..offset)),
                &mut None,
                window,
                cx,
            )
            .map(|text| text.trim().to_owned())
            .unwrap_or_default();
        self.completion_session.trigger_start_offset = Some(trigger_start);
        self.completion_session.query.clone_from(&query);
        let trigger = CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(query),
        };
        let task = provider.completions(&self.text, offset, trigger, window, cx);
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let items = match task.await? {
                CompletionResponse::Array(items) => items,
                CompletionResponse::List(list) => list.items,
            };
            editor.update_in(cx, |editor, window, cx| {
                if !editor.focus_handle.is_focused(window) {
                    return;
                }
                editor.completion_session.items = items;
                editor.completion_session.open = !editor.completion_session.items.is_empty();
                cx.notify();
            })?;
            Ok(())
        });
    }

    pub(crate) fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        self.completion_session.open = false;
        self.code_action_session.open = false;
        self._context_menu_task = Task::ready(Ok(()));
        cx.notify();
    }

    pub(crate) fn is_context_menu_open(&self, _cx: &App) -> bool {
        self.completion_session.open || self.code_action_session.open
    }

    /// Visual menu adapters consume actions first. Base keeps navigation
    /// routing presentation-independent; unconsumed actions fall through to
    /// normal editor movement.
    pub(crate) fn handle_action_for_context_menu(
        &mut self,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let closes_overlay =
            crate::input::Enter::is_primary(&*action) || action.partial_eq(&crate::input::Escape);
        let kind = if self.completion_session.open {
            Some(InputOverlayKind::Completion)
        } else if self.code_action_session.open {
            Some(InputOverlayKind::CodeAction)
        } else {
            None
        };
        let Some((kind, handler)) = kind.zip(self.overlay_action_handler.clone()) else {
            return false;
        };
        let handled = handler(kind, action, window, cx);
        if handled && closes_overlay {
            match kind {
                InputOverlayKind::Completion => self.completion_session.open = false,
                InputOverlayKind::CodeAction => self.code_action_session.open = false,
            }
            cx.notify();
        }
        handled
    }

    pub(crate) fn schedule_inline_completion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_inline_completion(cx);
        let Some(provider) = self.lsp.completion_provider.clone() else {
            return;
        };
        let offset = self.cursor();
        let text = self.text.clone();
        let debounce = provider.inline_completion_debounce();
        let executor = cx.background_executor().clone();
        self.inline_completion.task = cx.spawn_in(window, async move |editor, cx| {
            executor.timer(debounce).await;
            let task = editor.update_in(cx, |editor, window, cx| {
                (editor.cursor() == offset).then(|| {
                    provider.inline_completion(
                        &text,
                        offset,
                        InlineCompletionContext {
                            trigger_kind: InlineCompletionTriggerKind::Automatic,
                            selected_completion_info: None,
                        },
                        window,
                        cx,
                    )
                })
            })?;
            let Some(task) = task else {
                return Ok(InlineCompletionResponse::Array(Vec::new()));
            };
            let response = task.await?;
            editor.update_in(cx, |editor, _, cx| {
                if editor.cursor() != offset {
                    return;
                }
                editor.inline_completion.item = match response.clone() {
                    InlineCompletionResponse::Array(items) => items.into_iter().next(),
                    InlineCompletionResponse::List(list) => list.items.into_iter().next(),
                };
                cx.notify();
            })?;
            Ok(response)
        });
    }

    pub(crate) fn has_inline_completion(&self) -> bool {
        self.inline_completion.item.is_some()
    }

    pub(crate) fn clear_inline_completion(&mut self, cx: &mut Context<Self>) {
        self.inline_completion = InlineCompletion::default();
        cx.notify();
    }

    pub(crate) fn accept_inline_completion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(item) = self.inline_completion.item.take() else {
            return false;
        };
        let cursor = self.cursor();
        let range = self.range_to_utf16(&(cursor..cursor));
        self.replace_text_in_range_silent(Some(range), &item.insert_text, window, cx);
        true
    }

    pub fn apply_lsp_edits(
        &mut self,
        edits: &[lsp_types::TextEdit],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for edit in edits {
            let start = self.text.position_to_offset(&edit.range.start);
            let end = self.text.position_to_offset(&edit.range.end);
            let range = self.range_to_utf16(&(start..end));
            self.replace_text_in_range_silent(Some(range), &edit.new_text, window, cx);
        }
    }
}
