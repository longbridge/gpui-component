use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, RenderOnce, SharedString, Subscription, Window,
};

use super::{InputBaseState, InputEvent, TabSize, TextDecoration, TextDecorationCollection};

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
