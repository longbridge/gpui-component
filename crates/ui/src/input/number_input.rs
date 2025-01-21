use gpui::{
    actions, AppContext, EventEmitter, FocusHandle, FocusableView, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, Styled, Subscription, View, ViewContext,
    VisualContext,
};
use regex::Regex;

use crate::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{InputEvent, TextInput},
    prelude::FluentBuilder,
    ActiveTheme, IconName, Sizable, Size, StyledExt,
};

actions!(number_input, [Increment, Decrement]);

const KEY_CONTENT: &str = "NumberInput";

pub fn init(cx: &mut AppContext) {
    cx.bind_keys(vec![
        KeyBinding::new("up", Increment, Some(KEY_CONTENT)),
        KeyBinding::new("down", Decrement, Some(KEY_CONTENT)),
    ]);
}

pub struct NumberInput {
    input: View<TextInput>,
    _subscriptions: Vec<Subscription>,
}

impl NumberInput {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        // Default pattern for the number input.
        let pattern = Regex::new(r"^-?(\d+)?\.?(\d+)?$").unwrap();

        let input = cx.new_view(|cx| TextInput::new(cx).pattern(pattern).appearance(false));

        let _subscriptions = vec![cx.subscribe(&input, |_, _, event: &InputEvent, cx| {
            cx.emit(NumberInputEvent::Input(event.clone()));
        })];

        Self {
            input,
            _subscriptions,
        }
    }

    pub fn placeholder(
        self,
        placeholder: impl Into<SharedString>,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        self.input
            .update(cx, |input, _| input.set_placeholder(placeholder));
        self
    }

    pub fn set_placeholder(&self, text: impl Into<SharedString>, cx: &mut ViewContext<Self>) {
        self.input.update(cx, |input, _| {
            input.set_placeholder(text);
        });
    }

    pub fn pattern(self, pattern: regex::Regex, cx: &mut ViewContext<Self>) -> Self {
        self.input.update(cx, |input, _| input.set_pattern(pattern));
        self
    }

    pub fn set_size(self, size: Size, cx: &mut ViewContext<Self>) -> Self {
        self.input.update(cx, |input, cx| input.set_size(size, cx));
        self
    }

    pub fn small(self, cx: &mut ViewContext<Self>) -> Self {
        self.set_size(Size::Small, cx)
    }

    pub fn xsmall(self, cx: &mut ViewContext<Self>) -> Self {
        self.set_size(Size::XSmall, cx)
    }

    pub fn large(self, cx: &mut ViewContext<Self>) -> Self {
        self.set_size(Size::Large, cx)
    }

    pub fn set_value(&self, text: impl Into<SharedString>, cx: &mut ViewContext<Self>) {
        self.input.update(cx, |input, cx| input.set_text(text, cx))
    }

    pub fn set_disabled(&self, disabled: bool, cx: &mut ViewContext<Self>) {
        self.input
            .update(cx, |input, cx| input.set_disabled(disabled, cx));
    }

    pub fn increment(&mut self, cx: &mut ViewContext<Self>) {
        self.handle_increment(&Increment, cx);
    }

    pub fn decrement(&mut self, cx: &mut ViewContext<Self>) {
        self.handle_decrement(&Decrement, cx);
    }

    fn handle_increment(&mut self, _: &Increment, cx: &mut ViewContext<Self>) {
        self.on_step(StepAction::Increment, cx);
    }

    fn handle_decrement(&mut self, _: &Decrement, cx: &mut ViewContext<Self>) {
        self.on_step(StepAction::Decrement, cx);
    }

    fn on_step(&mut self, action: StepAction, cx: &mut ViewContext<Self>) {
        cx.emit(NumberInputEvent::Step(action));
    }
}

impl FocusableView for NumberInput {
    fn focus_handle(&self, cx: &AppContext) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

pub enum StepAction {
    Decrement,
    Increment,
}

pub enum NumberInputEvent {
    Input(InputEvent),
    Step(StepAction),
}

impl EventEmitter<NumberInputEvent> for NumberInput {}

impl Render for NumberInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let focused = self.input.focus_handle(cx).is_focused(cx);

        h_flex()
            .key_context(KEY_CONTENT)
            .on_action(cx.listener(Self::handle_increment))
            .on_action(cx.listener(Self::handle_decrement))
            .flex_1()
            .px_1()
            .gap_x_3()
            .bg(cx.theme().background)
            .border_color(cx.theme().input)
            .border_1()
            .rounded_md()
            .when(focused, |this| this.outline(cx))
            .child(
                Button::new("minus")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Minus)
                    .on_click(cx.listener(|this, _, cx| this.on_step(StepAction::Decrement, cx))),
            )
            .child(self.input.clone())
            .child(
                Button::new("plus")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Plus)
                    .on_click(cx.listener(|this, _, cx| this.on_step(StepAction::Increment, cx))),
            )
    }
}
