use gpui::{Window, ModelContext, Model, 
    actions, px, AppContext, EventEmitter, FocusHandle, FocusableView, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Pixels, Render, SharedString, Styled, Subscription,
      VisualContext,
};
use regex::Regex;

use crate::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{InputEvent, TextInput},
    prelude::FluentBuilder,
    ActiveTheme, IconName, Sizable, Size, StyleSized, StyledExt,
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
    size: Size,
    _subscriptions: Vec<Subscription>,
    _synced_size: bool,
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
            size: Size::default(),
            _synced_size: false,
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

    pub fn set_size(&mut self, size: Size, cx: &mut ViewContext<Self>) {
        self.size = size;
        self.sync_size_to_input_if_needed(cx);
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

    pub fn set_value(&self, text: impl Into<SharedString>, cx: &mut ViewContext<Self>) {
        self.input.update(cx, |input, cx| input.set_text(text, cx))
    }

    pub fn set_disabled(&self, disabled: bool, cx: &mut ViewContext<Self>) {
        self.input
            .update(cx, |input, cx| input.set_disabled(disabled, cx));
    }

    pub fn increment(&mut self, cx: &mut ViewContext<Self>) {
        self.on_action_increment(&Increment, cx);
    }

    pub fn decrement(&mut self, cx: &mut ViewContext<Self>) {
        self.on_action_decrement(&Decrement, cx);
    }

    fn on_action_increment(&mut self, _: &Increment, cx: &mut ViewContext<Self>) {
        self.on_step(StepAction::Increment, cx);
    }

    fn on_action_decrement(&mut self, _: &Decrement, cx: &mut ViewContext<Self>) {
        self.on_step(StepAction::Decrement, cx);
    }

    fn on_step(&mut self, action: StepAction, cx: &mut ViewContext<Self>) {
        if self.input.read(cx).disabled {
            return;
        }

        cx.emit(NumberInputEvent::Step(action));
    }

    fn sync_size_to_input_if_needed(&mut self, cx: &mut ViewContext<Self>) {
        if !self._synced_size {
            self.input
                .update(cx, |input, cx| input.set_size(self.size, cx));
            self._synced_size = true;
        }
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
impl Sizable for NumberInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Render for NumberInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let focused = self.input.focus_handle(cx).is_focused(cx);

        // Sync size to input at first.
        self.sync_size_to_input_if_needed(cx);
        const BUTTON_OFFSET: Pixels = px(-3.);
        let btn_size = match self.size {
            Size::XSmall | Size::Small => Size::XSmall,
            _ => Size::Small,
        };

        h_flex()
            .key_context(KEY_CONTENT)
            .on_action(cx.listener(Self::on_action_increment))
            .on_action(cx.listener(Self::on_action_decrement))
            .flex_1()
            .input_size(self.size)
            .bg(cx.theme().background)
            .border_color(cx.theme().input)
            .border_1()
            .rounded_md()
            .when(focused, |this| this.outline(cx))
            .child(
                Button::new("minus")
                    .ghost()
                    .with_size(btn_size)
                    .ml(BUTTON_OFFSET)
                    .icon(IconName::Minus)
                    .on_click(cx.listener(|this, _, cx| this.on_step(StepAction::Decrement, cx))),
            )
            .child(self.input.clone())
            .child(
                Button::new("plus")
                    .ghost()
                    .with_size(btn_size)
                    .mr(BUTTON_OFFSET)
                    .icon(IconName::Plus)
                    .on_click(cx.listener(|this, _, cx| this.on_step(StepAction::Increment, cx))),
            )
    }
}
