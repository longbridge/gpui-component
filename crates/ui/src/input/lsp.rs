use std::{cell::RefCell, ops::Range, rc::Rc};

use anyhow::Result;
use gpui::{App, Context, Entity, EntityInputHandler, SharedString, Task, Window};
use lsp_types::{
    request::Completion, CodeAction, CompletionContext, CompletionItem, CompletionResponse,
};
use rope::Rope;

use crate::input::{
    popovers::{CodeActionItem, CodeActionMenu, CompletionMenu, ContextMenu, HoverPopover},
    InputState, RopeExt,
};

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
    _hover_task: Task<Result<()>>,
}

impl Default for Lsp {
    fn default() -> Self {
        Self {
            completion_provider: None,
            code_action_providers: vec![],
            hover_provider: None,
            _hover_task: Task::ready(Ok(())),
        }
    }
}

/// A trait for providing code completions based on the current input state and context.
pub trait CompletionProvider {
    /// Fetches completions based on the given byte offset.
    fn completions(
        &self,
        text: &Rope,
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
    fn id(&self) -> SharedString;

    /// Fetches code actions for the specified range.
    fn code_actions(
        &self,
        state: Entity<InputState>,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>>;

    /// Performs the specified code action.
    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>>;
}

pub trait HoverProvider {
    /// Hover provider
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
    fn hover(
        &self,
        _text: &Rope,
        _offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>> {
        Task::ready(Ok(None))
    }
}

impl InputState {
    pub(crate) fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        self._context_menu_task = Task::ready(Ok(()));
        cx.notify();
    }

    pub(crate) fn is_context_menu_open(&self, cx: &App) -> bool {
        let Some(menu) = self.context_menu.as_ref() else {
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
        let Some(menu) = self.context_menu.as_ref() else {
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

        let Some(provider) = self.lsp.completion_provider.clone() else {
            return;
        };

        let start = range.end;
        let new_offset = self.cursor();

        if !provider.is_completion_trigger(start, new_text, cx) {
            return;
        }

        let menu = match self.context_menu.as_ref() {
            Some(ContextMenu::Completion(menu)) => Some(menu),
            _ => None,
        };

        // To create or get the existing completion menu.
        let menu = match menu {
            Some(menu) => menu.clone(),
            None => {
                let menu = CompletionMenu::new(cx.entity(), window, cx);
                self.context_menu = Some(ContextMenu::Completion(menu.clone()));
                menu
            }
        };

        let start_offset = menu.read(cx).trigger_start_offset.unwrap_or(start);
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
        _ = menu.update(cx, |menu, _| {
            menu.update_query(start_offset, query.clone());
        });

        let completion_context = CompletionContext {
            trigger_kind: lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(query),
        };

        let provider_responses =
            provider.completions(&self.text, start_offset, completion_context, window, cx);
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
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

    /// Show code actions for the cursor.
    pub(super) fn handle_code_action_trigger(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let providers = self.lsp.code_action_providers.clone();
        let menu = match self.context_menu.as_ref() {
            Some(ContextMenu::CodeAction(menu)) => Some(menu),
            _ => None,
        };

        let menu = match menu {
            Some(menu) => menu.clone(),
            None => {
                let menu = CodeActionMenu::new(cx.entity(), window, cx);
                self.context_menu = Some(ContextMenu::CodeAction(menu.clone()));
                menu
            }
        };

        let range = self.selected_range.start..self.selected_range.end;

        let state = cx.entity();
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut provider_responses = vec![];
            _ = cx.update(|window, cx| {
                for provider in providers {
                    let task = provider.code_actions(state.clone(), range.clone(), window, cx);
                    provider_responses.push((provider.id(), task));
                }
            });

            let mut code_actions: Vec<CodeActionItem> = vec![];
            for (provider_id, provider_responses) in provider_responses {
                if let Some(responses) = provider_responses.await.ok() {
                    code_actions.extend(responses.into_iter().map(|action| CodeActionItem {
                        provider_id: provider_id.clone(),
                        action,
                    }))
                }
            }

            if code_actions.is_empty() {
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
                        menu.show(editor.cursor(), code_actions, window, cx);
                    });

                    cx.notify();
                })
                .ok();

            Ok(())
        });
    }

    pub(crate) fn perform_code_action(
        &mut self,
        item: &CodeActionItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let providers = self.lsp.code_action_providers.clone();
        let Some(provider) = providers
            .iter()
            .find(|provider| provider.id() == item.provider_id)
        else {
            return;
        };

        let state = cx.entity();
        let task = provider.perform_code_action(state, item.action.clone(), true, window, cx);

        cx.spawn_in(window, async move |_, _| {
            let _ = task.await;
        })
        .detach();
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
            self.replace_text_in_range(Some(range_utf16), &edit.new_text, window, cx);
        }
    }

    /// Handle hover trigger LSP request.
    pub(super) fn handle_hover(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.lsp.hover_provider.clone() else {
            return;
        };

        if let Some(hover_popover) = self.hover_popover.as_ref() {
            if hover_popover.read(cx).is_same(offset) {
                return;
            }
        }

        // Currently not implemented.
        let task = provider.hover(&self.text, offset, window, cx);
        let word_range = self.text.word_range(offset).unwrap_or(offset..offset);
        let editor = cx.entity();
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let result = task.await?;

            _ = editor.update(cx, |editor, cx| match result {
                Some(hover) => {
                    let hover_popover = HoverPopover::new(cx.entity(), word_range, &hover, cx);
                    editor.hover_popover = Some(hover_popover);
                }
                None => {
                    editor.hover_popover = None;
                }
            });

            Ok(())
        });
    }
}
