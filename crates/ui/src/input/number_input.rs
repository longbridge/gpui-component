use gpui::{
    actions, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    SharedString, Styled, Subscription, Window,
};
use regex::Regex;

use crate::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{InputEvent, TextInput},
    ActiveTheme, IconName, Sizable, Size, StyleSized, StyledExt as _,
};

actions!(number_input, [Increment, Decrement]);

const KEY_CONTENT: &str = "NumberInput";

pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("up", Increment, Some(KEY_CONTENT)),
        KeyBinding::new("down", Decrement, Some(KEY_CONTENT)),
    ]);
}

pub struct NumberInput {
    input: Entity<TextInput>,
    size: Size,
    _subscriptions: Vec<Subscription>,
    _synced_size: bool,
}

impl NumberInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Default pattern for the number input.
        let pattern = Regex::new(r"^-?(\d+)?\.?(\d+)?$").unwrap();

        let input = cx.new(|cx| {
            TextInput::new(window, cx)
                .pattern(pattern)
                .no_gap()
                .appearance(false)
        });

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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        self.input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx)
        });
        self
    }

    pub fn set_size(&mut self, size: Size, window: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        self.sync_size_to_input_if_needed(window, cx);
    }

    pub fn set_placeholder(
        &self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input.update(cx, |input, cx| {
            input.set_placeholder(text, window, cx);
        });
    }

    pub fn pattern(
        self,
        pattern: regex::Regex,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        self.input
            .update(cx, |input, cx| input.set_pattern(pattern, window, cx));
        self
    }

    pub fn set_value(
        &self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input
            .update(cx, |input, cx| input.set_text(text, window, cx))
    }

    pub fn set_disabled(&self, disabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_disabled(disabled, window, cx));
    }

    pub fn increment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.on_action_increment(&Increment, window, cx);
    }

    pub fn decrement(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.on_action_decrement(&Decrement, window, cx);
    }

    fn on_action_increment(&mut self, _: &Increment, window: &mut Window, cx: &mut Context<Self>) {
        self.on_step(StepAction::Increment, window, cx);
    }

    fn on_action_decrement(&mut self, _: &Decrement, window: &mut Window, cx: &mut Context<Self>) {
        self.on_step(StepAction::Decrement, window, cx);
    }

    fn on_step(&mut self, action: StepAction, _: &mut Window, cx: &mut Context<Self>) {
        if self.input.read(cx).disabled {
            return;
        }

        cx.emit(NumberInputEvent::Step(action));
    }

    fn sync_size_to_input_if_needed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self._synced_size {
            self.input
                .update(cx, |input, cx| input.set_size(self.size, window, cx));
            self._synced_size = true;
        }
    }
}

impl Focusable for NumberInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.input.focus_handle(cx).is_focused(window);

        // Sync size to input at first.
        self.sync_size_to_input_if_needed(window, cx);
        let btn_size = match self.size {
            Size::XSmall | Size::Small => Size::Size(px(16.)),
            _ => Size::XSmall,
        };

        h_flex()
            .key_context(KEY_CONTENT)
            .on_action(cx.listener(Self::on_action_increment))
            .on_action(cx.listener(Self::on_action_decrement))
            .flex_1()
            .input_size(self.size)
            .px(match self.size {
                Size::XSmall => px(1.),
                Size::Small => px(2.),
                _ => px(3.),
            })
            .bg(cx.theme().background)
            .border_color(cx.theme().input)
            .border_1()
            .rounded(cx.theme().radius)
            .when(focused, |this| this.focused_border(cx))
            .child(
                Button::new("minus")
                    .ghost()
                    .with_size(btn_size)
                    .icon(IconName::Minus)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.on_step(StepAction::Decrement, window, cx)
                    })),
            )
            .child(self.input.clone())
            .child(
                Button::new("plus")
                    .ghost()
                    .with_size(btn_size)
                    .icon(IconName::Plus)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.on_step(StepAction::Increment, window, cx)
                    })),
            )
    }
}
