use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, RenderOnce, SharedString, Subscription, Window,
};

use super::{InputBaseState, InputEvent, Position};

struct TextareaOptions {
    placeholder: Option<SharedString>,
    default_value: Option<SharedString>,
    rows: Option<usize>,
    auto_grow: Option<(usize, usize)>,
    submit_on_enter: bool,
    soft_wrap: bool,
    searchable: bool,
    configured: bool,
}

impl Default for TextareaOptions {
    fn default() -> Self {
        Self {
            placeholder: None,
            default_value: None,
            rows: None,
            auto_grow: None,
            submit_on_enter: false,
            soft_wrap: true,
            searchable: false,
            configured: false,
        }
    }
}

/// State for editing ordinary multi-line text.
///
/// This deliberately exposes textarea concepts without exposing code-editor
/// facilities such as languages, diagnostics, folding, or LSP providers.
pub struct TextareaState {
    base: Entity<InputBaseState>,
    options: TextareaOptions,
    value: SharedString,
    _subscription: Subscription,
}

impl TextareaState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let base = cx.new(|cx| InputBaseState::new(window, cx).multi_line(true));
        let subscription = cx.subscribe(&base, |this, base, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.value = base.read(cx).value();
            }
            cx.emit(event.clone());
        });

        Self {
            base,
            options: TextareaOptions::default(),
            value: SharedString::default(),
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

    pub fn rows(mut self, rows: usize) -> Self {
        self.options.rows = Some(rows.max(1));
        self
    }

    /// Grow with the content from `min_rows` through `max_rows`.
    pub fn auto_grow(mut self, min_rows: usize, max_rows: usize) -> Self {
        let min_rows = min_rows.max(1);
        self.options.auto_grow = Some((min_rows, max_rows.max(min_rows)));
        self
    }

    /// Submit on Enter while Shift+Enter inserts a newline.
    pub fn submit_on_enter(mut self, submit: bool) -> Self {
        self.options.submit_on_enter = submit;
        self
    }

    pub fn soft_wrap(mut self, wrap: bool) -> Self {
        self.options.soft_wrap = wrap;
        self
    }

    pub fn searchable(mut self, searchable: bool) -> Self {
        self.options.searchable = searchable;
        self
    }

    pub fn value(&self) -> SharedString {
        self.value.clone()
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

    pub fn insert(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base
            .update(cx, |state, cx| state.insert(text, window, cx));
        self.value = self.base.read(cx).value();
    }

    pub fn replace(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base
            .update(cx, |state, cx| state.replace(text, window, cx));
        self.value = self.base.read(cx).value();
    }

    pub fn cursor_position(&self, cx: &App) -> Position {
        self.base.read(cx).cursor_position()
    }

    #[doc(hidden)]
    pub fn base_state(&self) -> &Entity<InputBaseState> {
        &self.base
    }

    fn configure(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.options.configured {
            return;
        }
        self.options.configured = true;

        let placeholder = self.options.placeholder.take();
        let default_value = self.options.default_value.take();
        let rows = self.options.rows;
        let auto_grow = self.options.auto_grow;
        let submit_on_enter = self.options.submit_on_enter;
        let soft_wrap = self.options.soft_wrap;
        let searchable = self.options.searchable;
        self.base.update(cx, |state, cx| {
            if let Some(placeholder) = placeholder {
                state.set_placeholder(placeholder, window, cx);
            }
            if let Some(value) = default_value {
                state.set_value(value, window, cx);
            }
            if let Some((min_rows, max_rows)) = auto_grow {
                state.set_auto_grow(min_rows, max_rows, cx);
            } else if let Some(rows) = rows {
                state.set_rows(rows, cx);
            }
            state.set_submit_on_enter(submit_on_enter, cx);
            state.set_soft_wrap(soft_wrap, window, cx);
            state.set_searchable(searchable, cx);
        });
    }

    #[doc(hidden)]
    pub fn prepare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.configure(window, cx);
    }
}

impl EventEmitter<InputEvent> for TextareaState {}

impl Focusable for TextareaState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.base.read(cx).focus_handle(cx)
    }
}

impl Render for TextareaState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.configure(window, cx);
        self.base.clone()
    }
}

/// An unstyled ordinary multi-line text input.
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
}

impl Textarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Textarea {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}
