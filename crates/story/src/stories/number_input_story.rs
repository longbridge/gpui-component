use gpui::{
    App, AppContext as _, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Render, Styled, Subscription, Window, px,
};
use regex::Regex;

use crate::section;
use gpui_component::{
    ActiveTheme, Disableable, IconName, Sizable,
    button::{Button, ButtonVariants},
    input::{InputEvent, InputState, MaskPattern, NumberInput, NumberInputEvent, StepAction},
    v_flex,
};

pub fn init(_: &mut App) {}

pub struct NumberInputStory {
    number_input1_value: i64,
    number_input1: Entity<InputState>,
    number_input2: Entity<InputState>,
    number_input3: Entity<InputState>,
    number_input4: Entity<InputState>,
    disabled_input: Entity<InputState>,

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
        // Opt out of internal stepping via `set_step(None)`, so the
        // NumberInput emits NumberInputEvent::Step and the subscriber is
        // responsible for updating the value (see `on_number_input_event`).
        let number_input1_value = 1;
        let number_input1 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Normal Integer")
                .default_value(number_input1_value.to_string())
        });
        number_input1.update(cx, |state, cx| state.set_step(None, window, cx));

        // With min, the NumberInput steps the value internally (step
        // default: 1) and clamps it to the range, no event handling needed.
        let number_input2 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Unsized Integer")
                .pattern(Regex::new(r"^\d+$").unwrap())
                .min(0.)
        });

        let number_input3 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Mask pattern")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
                .default_value("1234.56")
                .step(100.)
                .min(0.)
        });

        // The step varies by direction at the boundary 1.0: 0.1 going down,
        // 0.5 going up.
        let number_input4 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Styling")
                .default_value("0.9")
                .step_by(|value, action, _cx| match action {
                    StepAction::Increment => {
                        if value < 1.0 {
                            0.1
                        } else {
                            0.5
                        }
                    }
                    StepAction::Decrement => {
                        if value <= 1.0 {
                            0.1
                        } else {
                            0.5
                        }
                    }
                })
                .min(0.)
        });

        let disabled_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("100")
                .placeholder("Disabled")
        });

        let _subscriptions = vec![
            cx.subscribe_in(&number_input1, window, Self::on_input_event),
            cx.subscribe_in(&number_input1, window, Self::on_number_input_event),
            cx.subscribe_in(&number_input2, window, Self::on_input_event),
            cx.subscribe_in(&number_input3, window, Self::on_input_event),
            cx.subscribe_in(&number_input4, window, Self::on_input_event),
            cx.subscribe_in(&disabled_input, window, Self::on_input_event),
            cx.subscribe_in(&disabled_input, window, Self::on_number_input_event),
        ];

        Self {
            number_input1,
            number_input1_value,
            number_input2,
            number_input3,
            number_input4,
            disabled_input,
            _subscriptions,
        }
    }

    fn on_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                if state == &self.number_input1 {
                    if let Ok(value) = text.parse::<i64>() {
                        self.number_input1_value = value;
                    }
                }
                println!("Change: {}", text);
            }
            InputEvent::PressEnter { secondary, shift } => {
                println!("PressEnter secondary: {}, shift: {}", secondary, shift)
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
            NumberInputEvent::Step(step_action) => {
                if this == &self.number_input1 {
                    match step_action {
                        StepAction::Decrement => {
                            self.number_input1_value -= 1;
                        }
                        StepAction::Increment => {
                            self.number_input1_value += 1;
                        }
                    }
                    this.update(cx, |input, cx| {
                        input.set_value(self.number_input1_value.to_string(), window, cx);
                    });
                }
            }
        }
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
            .id("input-story")
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Normal Size")
                    .max_w(px(200.))
                    .child(NumberInput::new(&self.number_input1)),
            )
            .child(
                section("Disabled")
                    .max_w(px(200.))
                    .child(NumberInput::new(&self.disabled_input).disabled(true)),
            )
            .child(
                section("Small Size with suffix").max_w(px(200.)).child(
                    NumberInput::new(&self.number_input2)
                        .small()
                        .suffix(Button::new("info").ghost().icon(IconName::Info).xsmall()),
                ),
            )
            .child(
                section("With mask pattern")
                    .max_w(px(200.))
                    .child(NumberInput::new(&self.number_input3)),
            )
            .child(
                section("Without appearance").max_w(px(200.)).child(
                    NumberInput::new(&self.number_input4)
                        .appearance(false)
                        .bg(cx.theme().secondary)
                        .text_color(cx.theme().info),
                ),
            )
    }
}
