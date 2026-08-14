use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    Render, RenderOnce, SharedString, Subscription, Window,
};
use std::rc::Rc;

use super::{
    InputBaseState, InputEditorStyle, InputEvent, MaskPattern, NumberInputEvent, NumberStep,
};
use crate::StepAction;

type InputValidator = Rc<dyn Fn(&str, &mut App) -> bool>;
type InputStep = Rc<dyn Fn(f64, StepAction, &mut App) -> f64>;

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

    pub fn validate(mut self, validate: impl Fn(&str, &mut App) -> bool + 'static) -> Self {
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

    pub fn step_by(mut self, step: impl Fn(f64, StepAction, &mut App) -> f64 + 'static) -> Self {
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
                state.set_validator(move |value, cx| validator(value, cx), cx);
            }
            if let Some(step) = step {
                state.set_step(Some(step), window, cx);
            }
            if let Some(step_by) = step_by {
                state.set_step(
                    Some(NumberStep::by_value(move |value, action, cx| {
                        step_by(value, action, cx)
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

/// An unstyled single-line text input.
///
/// Applications that need a fully styled control can wrap this state with
/// their own presentation or use `gpui-component::Input`.
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Input {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{ParentElement as _, TestAppContext, VisualTestContext, div};

    use super::*;

    struct TestRoot(Entity<InputState>);

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().child(self.0.clone())
        }
    }

    #[gpui::test]
    fn facade_callbacks_do_not_reenter_input_state(cx: &mut TestAppContext) {
        let validation_calls = Rc::new(Cell::new(0));
        let step_calls = Rc::new(Cell::new(0));
        let mut input = None;

        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let validation_calls = validation_calls.clone();
                let step_calls = step_calls.clone();
                let state = cx.new(|cx| {
                    InputState::new(window, cx)
                        .validate(move |_, _| {
                            validation_calls.set(validation_calls.get() + 1);
                            true
                        })
                        .step_by(move |_, _, _| {
                            step_calls.set(step_calls.get() + 1);
                            1.
                        })
                });
                input = Some(state.clone());
                cx.new(|_| TestRoot(state))
            })
            .unwrap()
        });
        let input = input.unwrap();
        let mut cx = VisualTestContext::from_window(window.into(), cx);

        cx.update(|window, cx| {
            let _ = window.draw(cx);
            input.update(cx, |state, cx| {
                state.set_value("1", window, cx);
                state.replace_all("2", window, cx);
            });
        });

        let base = input.read_with(&cx, |state, _| state.base.clone());
        cx.update(|_, cx| {
            base.update(cx, |state, cx| {
                let step = state.number_step.clone().unwrap();
                assert_eq!(step.value(2., StepAction::Increment, cx), 1.);
            });
        });

        assert!(validation_calls.get() >= 2);
        assert_eq!(step_calls.get(), 1);
    }
}
