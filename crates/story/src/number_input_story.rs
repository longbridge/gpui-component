use gpui::{
    actions, App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, ParentElement as _, Render, Styled, Subscription, Window,
};
use regex::Regex;

use crate::section;
use gpui_component::{
    input::{InputEvent, NumberInput, NumberInputEvent, StepAction},
    v_flex, FocusableCycle, Sizable,
};

actions!(input_story, [Tab, TabPrev]);

const CONTEXT: &str = "NumberInputStory";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

pub struct NumberInputStory {
    number_input1_value: i64,
    number_input1: Entity<NumberInput>,
    number_input2: Entity<NumberInput>,
    number_input2_value: u64,

    _subscriptions: Vec<Subscription>,
}

impl super::Story for NumberInputStory {
    fn title() -> &'static str {
        "NumberInput"
    }

    fn description() -> &'static str {
        "NumberInput design to support + - to adjust the input value."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl NumberInputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let number_input1_value = 1;
        let number_input1 = cx.new(|cx| {
            let input = NumberInput::new(window, cx).placeholder("Number Input", window, cx);
            input.set_value(number_input1_value.to_string(), window, cx);
            input
        });

        let number_input2 = cx.new(|cx| {
            NumberInput::new(window, cx)
                .placeholder("Unsized Integer Number Input", window, cx)
                .pattern(Regex::new(r"^\d+$").unwrap(), window, cx)
                .small()
        });

        let _subscriptions = vec![
            cx.subscribe_in(&number_input1, window, Self::on_number_input1_event),
            cx.subscribe_in(&number_input2, window, Self::on_number_input2_event),
        ];

        Self {
            number_input1,
            number_input1_value,
            number_input2,
            number_input2_value: 0,
            _subscriptions,
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn on_number_input1_event(
        &mut self,
        _: &Entity<NumberInput>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Input(input_event) => match input_event {
                InputEvent::Change(text) => {
                    if let Ok(value) = text.parse::<i64>() {
                        self.number_input1_value = value;
                    }
                    println!("Change: {}", text);
                }
                InputEvent::PressEnter { secondary } => {
                    println!("PressEnter secondary: {}", secondary)
                }
                InputEvent::Focus => println!("Focus"),
                InputEvent::Blur => println!("Blur"),
            },
            NumberInputEvent::Step(step_action) => match step_action {
                StepAction::Decrement => {
                    self.number_input1_value = self.number_input1_value - 1;
                    self.number_input1.update(cx, |input, cx| {
                        input.set_value(self.number_input1_value.to_string(), window, cx);
                    });
                }
                StepAction::Increment => {
                    self.number_input1_value = self.number_input1_value + 1;
                    self.number_input1.update(cx, |input, cx| {
                        input.set_value(self.number_input1_value.to_string(), window, cx);
                    });
                }
            },
        }
    }

    fn on_number_input2_event(
        &mut self,
        _: &Entity<NumberInput>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Input(input_event) => match input_event {
                InputEvent::Change(text) => {
                    if let Ok(value) = text.parse::<u64>() {
                        self.number_input2_value = value;
                    }
                    println!("Change: {}", text);
                }
                InputEvent::PressEnter { secondary } => {
                    println!("PressEnter secondary: {}", secondary);
                }
                InputEvent::Focus => println!("Focus"),
                InputEvent::Blur => println!("Blur"),
            },
            NumberInputEvent::Step(step_action) => match step_action {
                StepAction::Decrement => {
                    if self.number_input2_value.le(&0) {
                        return;
                    }

                    self.number_input2_value = self.number_input2_value - 1;
                    self.number_input2.update(cx, |input, cx| {
                        input.set_value(self.number_input2_value.to_string(), window, cx);
                    });
                }
                StepAction::Increment => {
                    self.number_input2_value = self.number_input2_value + 1;
                    self.number_input2.update(cx, |input, cx| {
                        input.set_value(self.number_input2_value.to_string(), window, cx);
                    });
                }
            },
        }
    }
}

impl FocusableCycle for NumberInputStory {
    fn cycle_focus_handles(&self, _: &mut Window, _cx: &mut App) -> Vec<FocusHandle> {
        [].to_vec()
    }
}
impl Focusable for NumberInputStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.number_input1.focus_handle(cx)
    }
}

impl Render for NumberInputStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("input-story")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Normal Size")
                    .max_w_md()
                    .child(self.number_input1.clone()),
            )
            .child(
                section("Small Size")
                    .max_w_md()
                    .child(self.number_input2.clone()),
            )
    }
}
