use gpui::{
    actions, App, AppContext as _, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Render, Styled, Subscription, Window,
};
use regex::Regex;

use crate::section;
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{InputEvent, InputState, MaskPattern, NumberInput, NumberInputEvent, StepAction},
    v_flex, IconName, Sizable,
};

actions!(input_story, [Tab, TabPrev]);

pub fn init(_: &mut App) {}

pub struct NumberInputStory {
    number_input1_value: i64,
    number_input1: Entity<InputState>,
    number_input2: Entity<InputState>,
    number_input2_value: u64,
    number_input3: Entity<InputState>,
    number_input3_value: f64,

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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
            InputState::new(window, cx)
                .placeholder("Number Input")
                .default_value(number_input1_value.to_string())
        });

        let number_input2 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Unsized Integer Number Input")
                .pattern(Regex::new(r"^\d+$").unwrap())
        });

        let number_input3 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Number Input with mask pattern")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
        });

        let _subscriptions = vec![
            cx.subscribe_in(&number_input1, window, Self::on_input_event),
            cx.subscribe_in(&number_input1, window, Self::on_number_input_event),
            cx.subscribe_in(&number_input2, window, Self::on_input_event),
            cx.subscribe_in(&number_input2, window, Self::on_number_input_event),
            cx.subscribe_in(&number_input3, window, Self::on_input_event),
            cx.subscribe_in(&number_input3, window, Self::on_number_input_event),
        ];

        Self {
            number_input1,
            number_input1_value,
            number_input2,
            number_input2_value: 0,
            number_input3,
            number_input3_value: 0.0,
            _subscriptions,
        }
    }

    fn on_input_event(
        &mut self,
        this: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change(text) => {
                if this == &self.number_input1 {
                    if let Ok(value) = text.parse::<i64>() {
                        self.number_input1_value = value;
                    }
                } else if this == &self.number_input2 {
                    if let Ok(value) = text.parse::<u64>() {
                        self.number_input2_value = value;
                    }
                } else if this == &self.number_input3 {
                    if let Ok(value) = text.parse::<f64>() {
                        self.number_input3_value = value;
                    }
                }
                println!("Change: {}", text);
            }
            InputEvent::PressEnter { secondary } => {
                println!("PressEnter secondary: {}", secondary)
            }
            InputEvent::Focus => println!("Focus"),
            InputEvent::Blur => println!("Blur"),
        }
    }

    fn on_number_input_event(
        &mut self,
        this: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(step_action) => match step_action {
                StepAction::Decrement => {
                    if this == &self.number_input1 {
                        self.number_input1_value = self.number_input1_value - 1;
                        this.update(cx, |input, cx| {
                            input.set_value(self.number_input1_value.to_string(), window, cx);
                        });
                    } else if this == &self.number_input2 {
                        self.number_input2_value = self.number_input2_value.saturating_sub(1);
                        this.update(cx, |input, cx| {
                            input.set_value(self.number_input2_value.to_string(), window, cx);
                        });
                    } else if this == &self.number_input3 {
                        self.number_input3_value = self.number_input3_value - 1.0;
                        this.update(cx, |input, cx| {
                            input.set_value(self.number_input3_value.to_string(), window, cx);
                        });
                    }
                }
                StepAction::Increment => {
                    if this == &self.number_input1 {
                        self.number_input1_value = self.number_input1_value + 1;
                        this.update(cx, |input, cx| {
                            input.set_value(self.number_input1_value.to_string(), window, cx);
                        });
                    } else if this == &self.number_input2 {
                        self.number_input2_value = self.number_input2_value + 1;
                        this.update(cx, |input, cx| {
                            input.set_value(self.number_input2_value.to_string(), window, cx);
                        });
                    } else if this == &self.number_input3 {
                        self.number_input3_value = self.number_input3_value + 1.0;
                        this.update(cx, |input, cx| {
                            input.set_value(self.number_input3_value.to_string(), window, cx);
                        });
                    }
                }
            },
        }
    }
}

impl Focusable for NumberInputStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.number_input1.focus_handle(cx)
    }
}

impl Render for NumberInputStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("input-story")
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Normal Size")
                    .max_w_md()
                    .child(NumberInput::new(&self.number_input1)),
            )
            .child(
                section("Small Size with suffix").max_w_md().child(
                    NumberInput::new(&self.number_input2)
                        .small()
                        .suffix(Button::new("info").ghost().icon(IconName::Info).xsmall()),
                ),
            )
            .child(
                section("With mask pattern")
                    .max_w_md()
                    .child(NumberInput::new(&self.number_input3)),
            )
    }
}
