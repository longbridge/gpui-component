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
use crate::highlighter::HighlightTheme;

const DEFAULT_INLINE_COMPLETION_DEBOUNCE: Duration = Duration::from_millis(300);

pub type ShowDocumentHandler =
    Rc<dyn Fn(&lsp_types::ShowDocumentParams, &mut Window, &mut App) -> bool>;

pub trait DefinitionProvider {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>>;
}

pub trait HoverProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>>;
}

pub trait DocumentColorProvider {
    fn document_colors(
        &self,
        text: &Rope,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<ColorInformation>>>;
}

pub trait DocumentRangeSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend;

    fn semantic_tokens(
        &self,
        text: &Rope,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<SemanticTokens>>;
}

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

pub trait CodeActionProvider {
    fn id(&self) -> SharedString;

    fn code_actions(
        &self,
        state: Entity<InputState>,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>>;

    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>>;
}

#[derive(Clone, Debug)]
pub struct CodeActionItem {
    pub provider_id: SharedString,
    pub action: CodeAction,
}

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
    pub(super) item: Option<InlineCompletionItem>,
    pub(super) task: Task<Result<InlineCompletionResponse>>,
}

impl Default for InlineCompletion {
    fn default() -> Self {
        Self {
            item: None,
            task: Task::ready(Ok(InlineCompletionResponse::Array(vec![]))),
        }
    }
}

#[derive(Clone, Default)]
pub struct HoverDefinition {
    pub(super) symbol_range: Range<usize>,
    pub(super) locations: Rc<Vec<lsp_types::LocationLink>>,
    pub(super) last_location: Option<(Range<usize>, Rc<Vec<lsp_types::LocationLink>>)>,
}

impl HoverDefinition {
    pub(super) fn update(&mut self, range: Range<usize>, locations: Vec<lsp_types::LocationLink>) {
        self.clear();
        self.symbol_range = range;
        self.locations = Rc::new(locations);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
    pub(super) fn is_same(&self, offset: usize) -> bool {
        self.symbol_range.contains(&offset)
    }

    pub(super) fn clear(&mut self) {
        if !self.locations.is_empty() {
            self.last_location = Some((self.symbol_range.clone(), self.locations.clone()));
        }
        self.symbol_range = 0..0;
        self.locations = Rc::new(Vec::new());
    }
}

pub struct Lsp {
    pub completion_provider: Option<Rc<dyn CompletionProvider>>,
    pub code_action_providers: Vec<Rc<dyn CodeActionProvider>>,
    pub hover_provider: Option<Rc<dyn HoverProvider>>,
    pub definition_provider: Option<Rc<dyn DefinitionProvider>>,
    pub document_color_provider: Option<Rc<dyn DocumentColorProvider>>,
    pub semantic_tokens_provider: Option<Rc<dyn DocumentRangeSemanticTokensProvider>>,
    pub show_document: Option<ShowDocumentHandler>,
    pub completion_menu: CompletionMenuOptions,
    pub(super) document_colors: Vec<(lsp_types::Range, Hsla)>,
    pub(super) semantic_tokens: Vec<(lsp_types::Range, SharedString)>,
    pub(super) _hover_task: Task<Result<()>>,
    pub(super) _document_color_task: Task<()>,
    pub(super) _semantic_tokens_task: Task<()>,
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
    pub(super) fn update(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        self.update_document_colors(text, window, cx);
        self.update_semantic_tokens(text, window, cx);
    }

    fn update_document_colors(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.document_color_provider.clone() else {
            return;
        };
        let text = text.clone();
        let state = cx.entity();
        let executor = cx.background_executor().clone();
        self._document_color_task = cx.spawn_in(window, async move |_, cx| {
            executor.timer(Duration::from_millis(100)).await;
            let task = cx
                .update(|window, cx| provider.document_colors(&text, window, cx))
                .ok();
            if let Some(task) = task {
                if let Ok(colors) = task.await {
                    let _ = state.update(cx, |state, cx| {
                        let mut colors: Vec<_> = colors
                            .into_iter()
                            .map(|info| {
                                let color: Hsla = gpui::Rgba {
                                    r: info.color.red,
                                    g: info.color.green,
                                    b: info.color.blue,
                                    a: info.color.alpha,
                                }
                                .into();
                                (info.range, color)
                            })
                            .collect();
                        colors.sort_by_key(|(range, _)| range.start);
                        if colors != state.lsp.document_colors {
                            state.lsp.document_colors = colors;
                            cx.notify();
                        }
                    });
                }
            }
        });
    }

    fn update_semantic_tokens(
        &mut self,
        text: &Rope,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        let Some(provider) = self.semantic_tokens_provider.clone() else {
            return;
        };
        let legend = provider.legend();
        let text = text.clone();
        let range = 0..text.len();
        let state = cx.entity();
        let executor = cx.background_executor().clone();
        self._semantic_tokens_task = cx.spawn_in(window, async move |_, cx| {
            executor.timer(Duration::from_millis(100)).await;
            let task = cx
                .update(|window, cx| provider.semantic_tokens(&text, range, window, cx))
                .ok();
            if let Some(task) = task {
                if let Ok(tokens) = task.await {
                    let decoded = decode_semantic_tokens(&tokens, &legend);
                    let _ = state.update(cx, |state, cx| {
                        if decoded != state.lsp.semantic_tokens {
                            state.lsp.semantic_tokens = decoded;
                            cx.notify();
                        }
                    });
                }
            }
        });
    }

    pub(super) fn reset(&mut self) {
        self.document_colors.clear();
        self.semantic_tokens.clear();
        self._hover_task = Task::ready(Ok(()));
        self._document_color_task = Task::ready(());
        self._semantic_tokens_task = Task::ready(());
    }

    pub(super) fn document_colors_for_range(
        &self,
        text: &Rope,
        visible: &Range<usize>,
    ) -> Vec<(Range<usize>, Hsla)> {
        self.document_colors
            .iter()
            .filter_map(|(range, color)| {
                let start = text.position_to_offset(&range.start);
                let end = text.position_to_offset(&range.end);
                (start < visible.end && end > visible.start).then_some((start..end, *color))
            })
            .collect()
    }

    pub(super) fn semantic_tokens_for_range(
        &self,
        text: &Rope,
        visible: &Range<usize>,
        theme: &HighlightTheme,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.semantic_tokens
            .iter()
            .filter_map(|(range, name)| {
                let start = text.position_to_offset(&range.start);
                let end = text.position_to_offset(&range.end);
                if start >= end || start >= visible.end || end <= visible.start {
                    return None;
                }
                Some((start..end, theme.style(name.as_ref())?))
            })
            .collect()
    }
}

pub(super) fn decode_semantic_tokens(
    tokens: &SemanticTokens,
    legend: &SemanticTokensLegend,
) -> Vec<(lsp_types::Range, SharedString)> {
    let names: Vec<SharedString> = legend
        .token_types
        .iter()
        .map(|token| SharedString::from(token.as_str().to_owned()))
        .collect();
    let mut out = Vec::with_capacity(tokens.data.len());
    let (mut line, mut character) = (0, 0);
    for token in &tokens.data {
        if token.delta_line > 0 {
            line += token.delta_line;
            character = token.delta_start;
        } else {
            character += token.delta_start;
        }
        let Some(name) = names.get(token.token_type as usize) else {
            continue;
        };
        let start = lsp_types::Position::new(line, character);
        let end = lsp_types::Position::new(line, character + token.length);
        out.push((lsp_types::Range { start, end }, name.clone()));
    }
    out.sort_by_key(|(range, _)| range.start);
    out
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
        diagnostic: crate::highlighter::DiagnosticEntry,
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

    pub(super) fn handle_hover_definition(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self.lsp.definition_provider.clone() else {
            return;
        };
        if self.hover_definition.is_same(offset) {
            return;
        }
        let task = provider.definitions(&self.text, offset, window, cx);
        let fallback_range = self.text.word_range(offset).unwrap_or(offset..offset);
        let editor = cx.entity();
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let locations = task.await?;
            editor.update(cx, |editor, cx| {
                if locations.is_empty() {
                    editor.hover_definition.clear();
                } else {
                    let range = locations
                        .first()
                        .and_then(|location| location.origin_selection_range)
                        .map(|range| {
                            editor.text.position_to_offset(&range.start)
                                ..editor.text.position_to_offset(&range.end)
                        })
                        .unwrap_or(fallback_range);
                    editor.hover_definition.update(range, locations);
                }
                cx.notify();
            });
            Ok(())
        });
    }

    pub(super) fn handle_hover_popover(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selecting {
            return;
        }
        let Some(provider) = self.lsp.hover_provider.clone() else {
            return;
        };
        if self
            .hover_session
            .as_ref()
            .is_some_and(|session| session.symbol_range.contains(&offset))
        {
            return;
        }
        let task = provider.hover(&self.text, offset, window, cx);
        let fallback = self.text.word_range(offset).unwrap_or(offset..offset);
        let editor = cx.entity();
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            let hover = task.await?;
            editor.update(cx, |editor, cx| {
                editor.hover_session = hover.map(|hover| {
                    let symbol_range = hover
                        .range
                        .map(|range| {
                            editor.text.position_to_offset(&range.start)
                                ..editor.text.position_to_offset(&range.end)
                        })
                        .unwrap_or(fallback);
                    HoverSession {
                        symbol_range,
                        hover,
                    }
                });
                cx.notify();
            });
            Ok(())
        });
    }

    pub(super) fn handle_mouse_move(
        &mut self,
        offset: usize,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.modifiers.secondary() {
            self.handle_hover_definition(offset, window, cx);
        } else {
            self.hover_definition.clear();
            self.handle_hover_popover(offset, window, cx);
        }
        cx.notify();
    }

    pub fn clear_hover_state(&mut self, cx: &mut Context<Self>) {
        let changed = !self.hover_definition.is_empty() || self.hover_session.is_some();
        self.hover_definition.clear();
        self.hover_session = None;
        self.lsp._hover_task = Task::ready(Ok(()));
        if changed {
            cx.notify();
        }
    }

    pub(super) fn handle_click_hover_definition(
        &mut self,
        event: &MouseDownEvent,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !event.modifiers.secondary() || !self.hover_definition.is_same(offset) {
            return false;
        }
        let Some(location) = self.hover_definition.locations.first().cloned() else {
            return false;
        };
        self.go_to_definition(&location, window, cx);
        true
    }

    pub(super) fn on_action_go_to_definition(
        &mut self,
        _: &super::GoToDefinition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.cursor();
        let Some((range, locations)) = self.hover_definition.last_location.clone() else {
            return;
        };
        if !(range.start..=range.end).contains(&offset) {
            return;
        }
        if let Some(location) = locations.first() {
            self.go_to_definition(location, window, cx);
        }
    }

    fn go_to_definition(
        &mut self,
        location: &lsp_types::LocationLink,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let external = matches!(
            location.target_uri.scheme().map(|s| s.as_str()),
            Some("http" | "https")
        );
        if let Some(handler) = self.lsp.show_document.clone() {
            let params = lsp_types::ShowDocumentParams {
                uri: location.target_uri.clone(),
                external: Some(external),
                take_focus: Some(true),
                selection: Some(location.target_selection_range),
            };
            if handler(&params, window, cx) {
                return;
            }
        }
        if external {
            cx.open_url(&location.target_uri.to_string());
        } else {
            let start = self
                .text
                .position_to_offset(&location.target_selection_range.start);
            let end = self
                .text
                .position_to_offset(&location.target_selection_range.end);
            self.move_to(start, None, cx);
            self.select_to(end, cx);
        }
    }

    pub(super) fn on_action_toggle_code_actions(
        &mut self,
        _: &super::ToggleCodeActions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let providers = self.lsp.code_action_providers.clone();
        let range = self.selected_range.start..self.selected_range.end;
        let state = cx.entity();
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut tasks = Vec::new();
            let _ = cx.update(|window, cx| {
                tasks.extend(providers.iter().map(|provider| {
                    (
                        provider.id(),
                        provider.code_actions(state.clone(), range.clone(), window, cx),
                    )
                }));
            });
            let mut items = Vec::new();
            for (provider_id, task) in tasks {
                if let Ok(actions) = task.await {
                    items.extend(actions.into_iter().map(|action| CodeActionItem {
                        provider_id: provider_id.clone(),
                        action,
                    }));
                }
            }
            editor.update_in(cx, |editor, window, cx| {
                if !editor.focus_handle.is_focused(window) {
                    return;
                }
                editor.code_action_session.items = items;
                editor.code_action_session.open = !editor.code_action_session.items.is_empty();
                cx.notify();
            })?;
            Ok(())
        });
    }

    pub fn perform_code_action(
        &mut self,
        item: &CodeActionItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(provider) = self
            .lsp
            .code_action_providers
            .iter()
            .find(|provider| provider.id() == item.provider_id)
            .cloned()
        else {
            return;
        };
        let task = provider.perform_code_action(cx.entity(), item.action.clone(), true, window, cx);
        cx.spawn_in(window, async move |_, _| {
            let _ = task.await;
        })
        .detach();
    }

    pub(super) fn handle_completion_trigger(
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

    pub(super) fn hide_context_menu(&mut self, cx: &mut Context<Self>) {
        self.completion_session.open = false;
        self.code_action_session.open = false;
        self._context_menu_task = Task::ready(Ok(()));
        cx.notify();
    }

    pub(super) fn is_context_menu_open(&self, _cx: &App) -> bool {
        self.completion_session.open || self.code_action_session.open
    }

    /// Visual menu adapters consume actions first. Base keeps navigation
    /// routing presentation-independent; unconsumed actions fall through to
    /// normal editor movement.
    pub(super) fn handle_action_for_context_menu(
        &mut self,
        action: Box<dyn gpui::Action>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let closes_overlay =
            super::Enter::is_primary(&*action) || action.partial_eq(&super::Escape);
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

    pub(super) fn schedule_inline_completion(
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

    pub(super) fn has_inline_completion(&self) -> bool {
        self.inline_completion.item.is_some()
    }

    pub(super) fn clear_inline_completion(&mut self, cx: &mut Context<Self>) {
        self.inline_completion = InlineCompletion::default();
        cx.notify();
    }

    pub(super) fn accept_inline_completion(
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
