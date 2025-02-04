use gpui::{
    actions, div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyBinding, ParentElement as _,
    Render, SharedString, Styled, Window,
};
use regex::Regex;

use crate::section;
use ui::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{InputEvent, NumberInput, NumberInputEvent, OtpInput, TextInput},
    v_flex, FocusableCycle, IconName, Sizable,
};

actions!(input_story, [Tab, TabPrev]);

const CONTEXT: &str = "InputStory";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

pub struct InputStory {
    input1: Entity<TextInput>,
    input2: Entity<TextInput>,
    textarea: Entity<TextInput>,
    number_input1_value: i64,
    number_input1: Entity<NumberInput>,
    number_input2: Entity<NumberInput>,
    number_input2_value: u64,
    mash_input: Entity<TextInput>,
    disabled_input: Entity<TextInput>,
    prefix_input1: Entity<TextInput>,
    suffix_input1: Entity<TextInput>,
    both_input1: Entity<TextInput>,
    large_input: Entity<TextInput>,
    small_input: Entity<TextInput>,
    otp_masked: bool,
    otp_input: Entity<OtpInput>,
    otp_value: Option<SharedString>,
    otp_input_small: Entity<OtpInput>,
    otp_input_large: Entity<OtpInput>,
    opt_input_sized: Entity<OtpInput>,
}

impl super::Story for InputStory {
    fn title() -> &'static str {
        "Input"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl InputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input1 = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).cleanable();
            input.set_text(
                "Hello 世界，this is GPUI component, this is a long text.",
                window,
                cx,
            );
            input
        });
        cx.subscribe_in(&input1, window, Self::on_input_event)
            .detach();

        let input2 = cx.new(|cx| TextInput::new(window, cx).placeholder("Enter text here..."));
        cx.subscribe_in(&input2, window, Self::on_input_event)
            .detach();

        let textarea = cx.new(|cx| {
            let mut input = TextInput::new(window, cx)
                .multi_line()
                .rows(10)
                .placeholder("Enter text here...");
            input.set_text(
                unindent::unindent(
                    r#"Hello 世界，this is GPUI component.

                The GPUI Component is a collection of UI components for GPUI framework, including.

                Button, Input, Checkbox, Radio, Dropdown, Tab, and more...

                Here is an application that is built by using GPUI Component.

                > This application is still under development, not published yet.

                ![image](https://github.com/user-attachments/assets/559a648d-19df-4b5a-b563-b78cc79c8894)

                ![image](https://github.com/user-attachments/assets/5e06ad5d-7ea0-43db-8d13-86a240da4c8d)

                ## Demo

                If you want to see the demo, here is a some demo applications.
                "#,
                ),
                window,
                cx,
            );
            input
        });
        cx.subscribe_in(&textarea, window, Self::on_input_event)
            .detach();

        let number_input1_value = 1;
        let number_input1 = cx.new(|cx| {
            let input = NumberInput::new(window, cx).placeholder("Number Input", window, cx);
            input.set_value(number_input1_value.to_string(), window, cx);
            input
        });
        cx.subscribe_in(&number_input1, window, Self::on_number_input1_event)
            .detach();

        let number_input2 = cx.new(|cx| {
            NumberInput::new(window, cx)
                .placeholder("Unsized Integer Number Input", window, cx)
                .pattern(Regex::new(r"^\d+$").unwrap(), window, cx)
                .small()
        });

        cx.subscribe_in(&number_input2, window, Self::on_number_input2_event)
            .detach();

        let mask_input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx).cleanable();
            input.set_masked(true, window, cx);
            input.set_text("this-is-password", window, cx);
            input
        });

        let prefix_input1 = cx.new(|cx| {
            TextInput::new(window, cx)
                .prefix(|_, _| div().child(IconName::Search).ml_3())
                .placeholder("Search some thing...")
                .cleanable()
        });
        let suffix_input1 = cx.new(|cx| {
            TextInput::new(window, cx)
                .suffix(|_, _| div().child(IconName::Info).mr_3())
                .placeholder("This input only support [a-zA-Z0-9] characters.")
                .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
                .cleanable()
        });
        let both_input1 = cx.new(|cx| {
            TextInput::new(window, cx)
                .prefix(|_, _| div().child(IconName::Search).ml_3())
                .suffix(|_, _| div().child(IconName::Info).mr_3())
                .cleanable()
                .placeholder("This input have prefix and suffix.")
        });

        let otp_input = cx.new(|cx| OtpInput::new(6, window, cx).masked(true));
        cx.subscribe(&otp_input, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Change(text) => {
                this.otp_value = Some(text.clone());
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            input1,
            input2,
            textarea,
            number_input1,
            number_input1_value,
            number_input2,
            number_input2_value: 0,
            mash_input: mask_input,
            disabled_input: cx.new(|cx| {
                let mut input = TextInput::new(window, cx);
                input.set_text("This is disabled input", window, cx);
                input.set_disabled(true, window, cx);
                input
            }),
            large_input: cx.new(|cx| {
                TextInput::new(window, cx)
                    .large()
                    .placeholder("Large input")
            }),
            small_input: cx.new(|cx| {
                TextInput::new(window, cx)
                    .small()
                    .validate(|s| s.parse::<f32>().is_ok())
                    .placeholder("validate to limit float number.")
            }),
            prefix_input1,
            suffix_input1,
            both_input1,
            otp_masked: true,
            otp_input,
            otp_value: None,
            otp_input_small: cx.new(|cx| {
                OtpInput::new(6, window, cx)
                    .default_value("123456")
                    .masked(true)
                    .small()
                    .groups(1)
            }),
            otp_input_large: cx.new(|cx| {
                OtpInput::new(6, window, cx)
                    .groups(3)
                    .large()
                    .default_value("012345")
                    .masked(true)
            }),
            opt_input_sized: cx.new(|cx| {
                OtpInput::new(4, window, cx)
                    .groups(1)
                    .masked(true)
                    .default_value("654321")
                    .with_size(px(55.))
            }),
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
    }

    fn on_input_event(
        &mut self,
        _: &Entity<TextInput>,
        event: &InputEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change(text) => println!("Change: {}", text),
            InputEvent::PressEnter => println!("PressEnter"),
            InputEvent::Focus => println!("Focus"),
            InputEvent::Blur => println!("Blur"),
        };
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
                InputEvent::Change(text) => println!("Change: {}", text),
                InputEvent::PressEnter => println!("PressEnter"),
                InputEvent::Focus => println!("Focus"),
                InputEvent::Blur => println!("Blur"),
            },
            NumberInputEvent::Step(step_action) => match step_action {
                ui::input::StepAction::Decrement => {
                    self.number_input1_value = self.number_input1_value - 1;
                    self.number_input1.update(cx, |input, cx| {
                        input.set_value(self.number_input1_value.to_string(), window, cx);
                    });
                }
                ui::input::StepAction::Increment => {
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
                InputEvent::Change(text) => println!("Change: {}", text),
                InputEvent::PressEnter => println!("PressEnter"),
                InputEvent::Focus => println!("Focus"),
                InputEvent::Blur => println!("Blur"),
            },
            NumberInputEvent::Step(step_action) => match step_action {
                ui::input::StepAction::Decrement => {
                    if self.number_input2_value.le(&0) {
                        return;
                    }

                    self.number_input2_value = self.number_input2_value - 1;
                    self.number_input2.update(cx, |input, cx| {
                        input.set_value(self.number_input2_value.to_string(), window, cx);
                    });
                }
                ui::input::StepAction::Increment => {
                    self.number_input2_value = self.number_input2_value + 1;
                    self.number_input2.update(cx, |input, cx| {
                        input.set_value(self.number_input2_value.to_string(), window, cx);
                    });
                }
            },
        }
    }

    fn toggle_opt_masked(&mut self, _: &bool, window: &mut Window, cx: &mut Context<Self>) {
        self.otp_masked = !self.otp_masked;
        self.otp_input.update(cx, |input, cx| {
            input.set_masked(self.otp_masked, window, cx)
        });
        self.otp_input_small.update(cx, |input, cx| {
            input.set_masked(self.otp_masked, window, cx)
        });
        self.otp_input_large.update(cx, |input, cx| {
            input.set_masked(self.otp_masked, window, cx)
        });
        self.opt_input_sized.update(cx, |input, cx| {
            input.set_masked(self.otp_masked, window, cx)
        });
    }
}

impl FocusableCycle for InputStory {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        [
            self.input1.focus_handle(cx),
            self.input2.focus_handle(cx),
            self.disabled_input.focus_handle(cx),
            self.mash_input.focus_handle(cx),
            self.prefix_input1.focus_handle(cx),
            self.both_input1.focus_handle(cx),
            self.suffix_input1.focus_handle(cx),
            self.large_input.focus_handle(cx),
            self.small_input.focus_handle(cx),
            self.otp_input.focus_handle(cx),
        ]
        .to_vec()
    }
}
impl Focusable for InputStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.input1.focus_handle(cx)
    }
}

impl Render for InputStory {
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
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(
                        section("Normal Input", cx)
                            .child(self.input1.clone())
                            .child(self.input2.clone())
                            .child(
                                v_flex()
                                    .gap_y_4()
                                    .w_full()
                                    .child("Number Input")
                                    .child(self.number_input1.clone())
                                    .child(self.number_input2.clone()),
                            ),
                    )
                    .child(section("Textarea", cx).child(self.textarea.clone()))
                    .child(
                        section("Input State", cx)
                            .child(self.disabled_input.clone())
                            .child(self.mash_input.clone()),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(
                        section("Prefix and Suffix", cx)
                            .child(self.prefix_input1.clone())
                            .child(self.both_input1.clone())
                            .child(self.suffix_input1.clone()),
                    )
                    .child(
                        section("Input Size", cx)
                            .child(self.large_input.clone())
                            .child(self.small_input.clone()),
                    ),
            )
            .child(
                section(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child("OTP Input")
                        .child(
                            Checkbox::new("otp-mask")
                                .label("Masked")
                                .checked(self.otp_masked)
                                .on_click(cx.listener(Self::toggle_opt_masked)),
                        ),
                    cx,
                )
                .child(
                    v_flex()
                        .gap_3()
                        .child(self.otp_input_small.clone())
                        .child(self.otp_input.clone())
                        .when_some(self.otp_value.clone(), |this, otp| {
                            this.child(format!("Your OTP: {}", otp))
                        })
                        .child(self.otp_input_large.clone())
                        .child(self.opt_input_sized.clone()),
                ),
            )
            .child(
                h_flex()
                    .items_center()
                    .w_full()
                    .gap_3()
                    .child(
                        Button::new("btn-submit")
                            .flex_1()
                            .with_variant(ButtonVariant::Primary)
                            .label("Submit")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.dispatch_action(Box::new(Tab), cx)
                            })),
                    )
                    .child(
                        Button::new("btn-cancel")
                            .flex_1()
                            .label("Cancel")
                            .into_element(),
                    ),
            )
    }
}
