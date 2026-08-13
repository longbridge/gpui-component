use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, SharedString, Subscription, Window,
};
use std::rc::Rc;

use super::{
    InputBaseState, InputEditorStyle, InputEvent, MaskPattern, NumberInputEvent, NumberStep,
    Position, TabSize, TextDecoration, TextDecorationCollection,
};
use crate::StepAction;

type InputValidator = Rc<dyn Fn(&str, &mut Context<InputState>) -> bool>;
type InputStep = Rc<dyn Fn(f64, StepAction, &mut Context<InputState>) -> f64>;

#[derive(Default)]
struct InputOptions {
    placeholder: Option<SharedString>,
    default_value: Option<SharedString>,
    masked: bool,
    clean_on_escape: bool,
    submit_on_enter: bool,
    mask_pattern: Option<MaskPattern>,
    pattern: Option<regex::Regex>,
    validator: Option<InputValidator>,
    step: Option<NumberStep>,
    step_by: Option<InputStep>,
    min: Option<f64>,
    max: Option<f64>,
    context_menu: Option<bool>,
    editor_style: Option<InputEditorStyle>,
    configured: bool,
}

/// State for a single-line text input.
///
/// Multi-line layout, auto-grow, and code-editor configuration deliberately do
/// not exist on this type. Use [`TextareaState`] or [`EditorState`] instead.
pub struct InputState {
    base: Entity<InputBaseState>,
    options: InputOptions,
    value: SharedString,
    unmasked_value: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl InputState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let base = cx.new(|cx| InputBaseState::new(window, cx));
        let input_subscription = cx.subscribe(&base, |this, base, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.value = base.read(cx).value();
                this.unmasked_value = base.read(cx).unmask_value();
            }
            cx.emit(event.clone());
        });
        let number_subscription = cx.subscribe(&base, |_, _, event: &NumberInputEvent, cx| {
            cx.emit(event.clone());
        });
        Self {
            base,
            options: InputOptions::default(),
            value: SharedString::default(),
            unmasked_value: SharedString::default(),
            _subscriptions: vec![input_subscription, number_subscription],
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

    pub fn masked(mut self, masked: bool) -> Self {
        self.options.masked = masked;
        self
    }

    pub fn clean_on_escape(mut self) -> Self {
        self.options.clean_on_escape = true;
        self
    }

    pub fn submit_on_enter(mut self, submit: bool) -> Self {
        self.options.submit_on_enter = submit;
        self
    }

    pub fn mask_pattern(mut self, pattern: impl Into<MaskPattern>) -> Self {
        self.options.mask_pattern = Some(pattern.into());
        self
    }

    pub fn pattern(mut self, pattern: regex::Regex) -> Self {
        self.options.pattern = Some(pattern);
        self
    }

    pub fn validate(
        mut self,
        validate: impl Fn(&str, &mut Context<Self>) -> bool + 'static,
    ) -> Self {
        self.options.validator = Some(Rc::new(validate));
        self
    }

    pub fn context_menu(mut self, enabled: bool) -> Self {
        self.options.context_menu = Some(enabled);
        self
    }

    /// Supplies the visual metrics and colors used by the unstyled base input.
    pub fn set_editor_style(&mut self, style: InputEditorStyle) {
        self.options.editor_style = Some(style);
    }

    pub fn step(mut self, step: impl Into<NumberStep>) -> Self {
        self.options.step = Some(step.into());
        self
    }

    pub fn step_by(
        mut self,
        step: impl Fn(f64, StepAction, &mut Context<Self>) -> f64 + 'static,
    ) -> Self {
        self.options.step_by = Some(Rc::new(step));
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.options.min = Some(min);
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.options.max = Some(max);
        self
    }

    pub fn value(&self) -> SharedString {
        self.value.clone()
    }

    pub fn selected_value(&self, cx: &App) -> SharedString {
        self.base.read(cx).selected_value()
    }

    pub fn unmask_value(&self) -> SharedString {
        self.unmasked_value.clone()
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
        self.unmasked_value = self.base.read(cx).unmask_value();
    }

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
        self.base.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx)
        });
    }

    pub fn set_loading(&mut self, loading: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.base
            .update(cx, |state, cx| state.set_loading(loading, window, cx));
    }

    pub fn set_masked(&mut self, masked: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.base
            .update(cx, |state, cx| state.set_masked(masked, window, cx));
    }

    pub fn clean(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.base.update(cx, |state, cx| state.clean(window, cx));
        self.value = SharedString::default();
    }

    pub fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.base
            .update(cx, |state, cx| state.select_all(window, cx));
    }

    pub fn set_step(
        &mut self,
        step: impl Into<Option<NumberStep>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.base
            .update(cx, |state, cx| state.set_step(step, window, cx));
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
        let masked = self.options.masked;
        let clean_on_escape = self.options.clean_on_escape;
        let submit_on_enter = self.options.submit_on_enter;
        let mask_pattern = self.options.mask_pattern.take();
        let pattern = self.options.pattern.take();
        let validator = self.options.validator.take();
        let step = self.options.step.take();
        let step_by = self.options.step_by.take();
        let min = self.options.min;
        let max = self.options.max;
        let context_menu = self.options.context_menu;
        let editor_style = self.options.editor_style.take();
        let entity = cx.entity().downgrade();
        self.base.update(cx, |state, cx| {
            if let Some(placeholder) = placeholder {
                state.set_placeholder(placeholder, window, cx);
            }
            if let Some(value) = default_value {
                state.set_value(value, window, cx);
            }
            if masked {
                state.set_masked(true, window, cx);
            }
            if clean_on_escape {
                state.set_clean_on_escape(true);
            }
            state.set_submit_on_enter(submit_on_enter, cx);
            if let Some(pattern) = mask_pattern {
                state.set_mask_pattern(pattern, window, cx);
            }
            if let Some(pattern) = pattern {
                state.set_pattern(pattern, window, cx);
            }
            if let Some(validator) = validator {
                let entity = entity.clone();
                state.set_validator(
                    move |value, cx| {
                        entity
                            .update(cx, |_, cx| validator(value, cx))
                            .unwrap_or(false)
                    },
                    cx,
                );
            }
            if let Some(step) = step {
                state.set_step(Some(step), window, cx);
            }
            if let Some(step_by) = step_by {
                let entity = entity.clone();
                state.set_step(
                    Some(NumberStep::by_value(move |value, action, cx| {
                        entity
                            .update(cx, |_, cx| step_by(value, action, cx))
                            .unwrap_or_default()
                    })),
                    window,
                    cx,
                );
            }
            state.set_min(min, window, cx);
            state.set_max(max, window, cx);
            if let Some(context_menu) = context_menu {
                state.set_context_menu_enabled(context_menu);
            }
            if let Some(editor_style) = editor_style {
                state.set_editor_style(editor_style);
            }
        });
        self.value = self.base.read(cx).value();
        self.unmasked_value = self.base.read(cx).unmask_value();
    }
}

impl EventEmitter<InputEvent> for InputState {}
impl EventEmitter<NumberInputEvent> for InputState {}

impl Focusable for InputState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.base.read(cx).focus_handle(cx)
    }
}

impl Render for InputState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare(window, cx);
        self.base.clone()
    }
}

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
