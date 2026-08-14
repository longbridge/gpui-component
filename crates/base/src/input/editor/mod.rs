use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, RenderOnce, SharedString, Subscription, Window,
};
use ropey::Rope;

use super::{
    DiagnosticSet, InputBaseState, InputEvent, InputHighlighterFactory, Lsp, Position, TabSize,
    TextDecoration, TextDecorationCollection,
};

/// State for source-code editing.
///
/// Editor-specific configuration lives here so ordinary inputs and textareas
/// do not need to expose languages, line numbers, folding, diagnostics, or LSP.
#[derive(Default)]
struct EditorOptions {
    placeholder: Option<SharedString>,
    default_value: Option<SharedString>,
    tab_size: Option<TabSize>,
    line_number: Option<bool>,
    folding: Option<bool>,
    indent_guides: Option<bool>,
    soft_wrap: Option<bool>,
    searchable: Option<bool>,
    scroll_beyond_last_line: Option<Option<usize>>,
    cursor_surrounding_lines: Option<Option<usize>>,
    show_whitespaces: bool,
    configured: bool,
}

pub struct EditorState {
    base: Entity<InputBaseState>,
    value: SharedString,
    options: EditorOptions,
    _subscription: Subscription,
}

impl EditorState {
    pub fn new(
        language: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let language = language.into();
        let base = cx.new(|cx| InputBaseState::new(window, cx).code_editor(language));
        // Hand the engine a way back to us, so the LSP providers receive an
        // `Entity<EditorState>` rather than the engine itself.
        let owner = cx.weak_entity();
        base.update(cx, |state, _| state.set_editor_owner(owner));
        let subscription = cx.subscribe(&base, |this, base, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.value = base.read(cx).value();
            }
            cx.emit(event.clone());
        });
        Self {
            base,
            value: SharedString::default(),
            options: EditorOptions::default(),
            _subscription: subscription,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        let value = value.into();
        self.value = value.clone();
        self.options.default_value = Some(value);
        self
    }

    pub fn tab_size(mut self, tab_size: TabSize) -> Self {
        self.options.tab_size = Some(tab_size);
        self
    }

    pub fn line_number(mut self, line_number: bool) -> Self {
        self.options.line_number = Some(line_number);
        self
    }

    pub fn folding(mut self, folding: bool) -> Self {
        self.options.folding = Some(folding);
        self
    }

    pub fn show_whitespaces(mut self, show: bool) -> Self {
        self.options.show_whitespaces = show;
        self
    }

    /// Show indent guides, default is `true`.
    pub fn indent_guides(mut self, indent_guides: bool) -> Self {
        self.options.indent_guides = Some(indent_guides);
        self
    }

    /// Wrap long lines instead of scrolling horizontally.
    pub fn soft_wrap(mut self, wrap: bool) -> Self {
        self.options.soft_wrap = Some(wrap);
        self
    }

    /// Enable the search panel, default is `true` for an editor.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.options.searchable = Some(searchable);
        self
    }

    /// Number of empty rows kept scrollable past the last line.
    pub fn scroll_beyond_last_line(mut self, rows: Option<usize>) -> Self {
        self.options.scroll_beyond_last_line = Some(rows);
        self
    }

    /// Minimum number of lines kept between the cursor and the viewport edge.
    pub fn cursor_surrounding_lines(mut self, lines: Option<usize>) -> Self {
        self.options.cursor_surrounding_lines = Some(lines);
        self
    }

    pub fn value(&self) -> SharedString {
        self.value.clone()
    }

    /// Return the text [`Rope`] of the editor.
    ///
    /// Borrowed from the engine rather than mirrored, so it cannot go stale.
    pub fn text<'a>(&self, cx: &'a App) -> &'a Rope {
        self.base.read(cx).text()
    }

    /// Return the byte offset of the cursor.
    ///
    /// Read from the engine rather than mirrored: the cursor moves without a
    /// text change, and [`InputEvent`] has no event for it, so there is no
    /// point at which a copy could be kept in step.
    pub fn cursor(&self, cx: &App) -> usize {
        self.base.read(cx).cursor()
    }

    /// Return the (0-based) [`Position`] of the cursor.
    ///
    /// Read live, for the reason given on [`Self::cursor`].
    pub fn cursor_position(&self, cx: &App) -> Position {
        self.base.read(cx).cursor_position()
    }

    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle(cx).focus(window, cx);
    }

    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        self.value = value.clone();
        self.base
            .update(cx, |state, cx| state.set_value(value, window, cx));
    }

    /// Replace the entire text while preserving undo history.
    ///
    /// Unlike [`Self::set_value`], the user can undo this change.
    pub fn replace_all(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        self.base
            .update(cx, |state, cx| state.replace_all(value, window, cx));
        self.value = self.base.read(cx).value();
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let placeholder = placeholder.into();
        self.base.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx)
        });
    }

    /// Move the cursor to a (0-based) [`Position`] and focus the editor.
    pub fn set_cursor_position(
        &mut self,
        position: impl Into<Position>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let position = position.into();
        self.base.update(cx, |state, cx| {
            state.set_cursor_position(position, window, cx)
        });
    }

    pub fn set_line_number(
        &mut self,
        line_number: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base.update(cx, |state, cx| {
            state.set_line_number(line_number, window, cx)
        });
    }

    pub fn set_folding(&mut self, folding: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.base
            .update(cx, |state, cx| state.set_folding(folding, window, cx));
    }

    pub fn set_indent_guides(
        &mut self,
        indent_guides: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base.update(cx, |state, cx| {
            state.set_indent_guides(indent_guides, window, cx)
        });
    }

    pub fn set_soft_wrap(&mut self, wrap: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.base
            .update(cx, |state, cx| state.set_soft_wrap(wrap, window, cx));
    }

    pub fn set_show_whitespaces(
        &mut self,
        show: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base
            .update(cx, |state, cx| state.set_show_whitespaces(show, window, cx));
    }

    pub fn set_scroll_beyond_last_line(
        &mut self,
        rows: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base.update(cx, |state, cx| {
            state.set_scroll_beyond_last_line(rows, window, cx)
        });
    }

    pub fn set_cursor_surrounding_lines(
        &mut self,
        lines: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base.update(cx, |state, cx| {
            state.set_cursor_surrounding_lines(lines, window, cx)
        });
    }

    /// Switch the language used for syntax highlighting.
    pub fn set_highlighter(&mut self, language: impl Into<SharedString>, cx: &mut Context<Self>) {
        let language = language.into();
        self.base
            .update(cx, |state, cx| state.set_highlighter(language, cx));
    }

    /// Install the parser/highlighter adapter used for syntax highlighting.
    pub fn set_highlighter_factory(
        &mut self,
        factory: InputHighlighterFactory,
        cx: &mut Context<Self>,
    ) {
        self.base
            .update(cx, |state, cx| state.set_highlighter_factory(factory, cx));
    }

    /// Mutate the editor's diagnostics.
    ///
    /// The closure is skipped when the editor has no diagnostic set.
    pub fn update_diagnostics(
        &mut self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut DiagnosticSet),
    ) {
        self.base.update(cx, |state, cx| {
            if let Some(diagnostics) = state.diagnostics_mut() {
                f(diagnostics);
                cx.notify();
            }
        });
    }

    /// Apply a list of [`lsp_types::TextEdit`] to mutate the text.
    pub fn apply_lsp_edits(
        &mut self,
        text_edits: &Vec<lsp_types::TextEdit>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base.update(cx, |state, cx| {
            state.apply_lsp_edits(text_edits, window, cx)
        });
        self.value = self.base.read(cx).value();
    }

    /// Configure the LSP providers of this editor.
    ///
    /// ```ignore
    /// editor.update_lsp(cx, |lsp| {
    ///     lsp.completion_provider = Some(provider.clone());
    ///     lsp.hover_provider = Some(provider.clone());
    /// });
    /// ```
    pub fn update_lsp(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Lsp)) {
        self.base.update(cx, |state, cx| {
            f(&mut state.lsp);
            cx.notify();
        });
    }

    pub fn create_decorations_collection(
        &mut self,
        decorations: Vec<TextDecoration>,
        cx: &mut Context<Self>,
    ) -> TextDecorationCollection {
        self.base.update(cx, |state, cx| {
            state.create_decorations_collection(decorations, cx)
        })
    }

    #[doc(hidden)]
    pub fn base_state(&self) -> &Entity<InputBaseState> {
        &self.base
    }

    #[doc(hidden)]
    pub fn prepare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.options.configured {
            return;
        }
        self.options.configured = true;
        let placeholder = self.options.placeholder.take();
        let default_value = self.options.default_value.take();
        let tab_size = self.options.tab_size;
        let line_number = self.options.line_number;
        let folding = self.options.folding;
        let indent_guides = self.options.indent_guides;
        let soft_wrap = self.options.soft_wrap;
        let searchable = self.options.searchable;
        let scroll_beyond_last_line = self.options.scroll_beyond_last_line;
        let cursor_surrounding_lines = self.options.cursor_surrounding_lines;
        let show_whitespaces = self.options.show_whitespaces;
        self.base.update(cx, |state, cx| {
            if let Some(placeholder) = placeholder {
                state.set_placeholder(placeholder, window, cx);
            }
            if let Some(value) = default_value {
                state.set_value(value, window, cx);
            }
            if let Some(tab_size) = tab_size {
                state.set_tab_size(tab_size, cx);
            }
            if let Some(line_number) = line_number {
                state.set_line_number(line_number, window, cx);
            }
            if let Some(folding) = folding {
                state.set_folding(folding, window, cx);
            }
            if let Some(indent_guides) = indent_guides {
                state.set_indent_guides(indent_guides, window, cx);
            }
            if let Some(wrap) = soft_wrap {
                state.set_soft_wrap(wrap, window, cx);
            }
            if let Some(searchable) = searchable {
                state.set_searchable(searchable, cx);
            }
            if let Some(rows) = scroll_beyond_last_line {
                state.set_scroll_beyond_last_line(rows, window, cx);
            }
            if let Some(lines) = cursor_surrounding_lines {
                state.set_cursor_surrounding_lines(lines, window, cx);
            }
            state.set_show_whitespaces(show_whitespaces, window, cx);
        });
    }
}

impl EventEmitter<InputEvent> for EditorState {}

impl Focusable for EditorState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.base.read(cx).focus_handle(cx)
    }
}

impl Render for EditorState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare(window, cx);
        self.base.clone()
    }
}
/// An unstyled source-code editor.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
}

impl Editor {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}
