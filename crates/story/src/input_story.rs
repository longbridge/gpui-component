use gpui::{
    div, App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement as _, Render, Styled, Subscription, Window,
};

use crate::section;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{InputEvent, InputState, MaskPattern, TextInput},
    v_flex, ContextModal, Icon, IconName, Sizable,
};

pub struct InputStory {
    focus_handle: FocusHandle,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    input_esc: Entity<InputState>,
    mask_input: Entity<InputState>,
    disabled_input: Entity<InputState>,
    prefix_input1: Entity<InputState>,
    suffix_input1: Entity<InputState>,
    both_input1: Entity<InputState>,
    large_input: Entity<InputState>,
    small_input: Entity<InputState>,
    phone_input: Entity<InputState>,
    mask_input2: Entity<InputState>,
    currency_input: Entity<InputState>,

    _subscriptions: Vec<Subscription>,
}

impl super::Story for InputStory {
    fn title() -> &'static str {
        "Input"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl InputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input1 = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("Hello 世界，this is GPUI component, this is a long text.")
        });

        let input2 = cx.new(|cx| InputState::new(window, cx).placeholder("Enter text here..."));
        let input_esc = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter text and clear it by pressing ESC")
                .clean_on_escape()
        });

        let mask_input = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .default_value("this-is-password")
        });

        let prefix_input1 =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search some thing..."));
        let suffix_input1 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("This input only support [a-zA-Z0-9] characters.")
                .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
        });
        let both_input1 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("This input have prefix and suffix.")
        });

        let phone_input = cx.new(|cx| InputState::new(window, cx).mask_pattern("(999)-999-9999"));
        let mask_input2 = cx.new(|cx| InputState::new(window, cx).mask_pattern("AAA-###-AAA"));
        let currency_input = cx.new(|cx| {
            InputState::new(window, cx).mask_pattern(MaskPattern::Number {
                separator: Some(','),
                fraction: Some(3),
            })
        });

        let _subscriptions = vec![
            cx.subscribe_in(&input1, window, Self::on_input_event),
            cx.subscribe_in(&input2, window, Self::on_input_event),
            cx.subscribe_in(&phone_input, window, Self::on_input_event),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            input1,
            input2,
            input_esc,
            mask_input,
            disabled_input: cx
                .new(|cx| InputState::new(window, cx).default_value("This is disabled input")),
            large_input: cx.new(|cx| InputState::new(window, cx).placeholder("Large input")),
            small_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .validate(|s| s.parse::<f32>().is_ok())
                    .placeholder("validate to limit float number.")
            }),
            prefix_input1,
            suffix_input1,
            both_input1,
            phone_input,
            mask_input2,
            currency_input,
            _subscriptions,
        }
    }

    fn on_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change(text) => println!("Change: {}", text),
            InputEvent::PressEnter { secondary } => println!("PressEnter secondary: {}", secondary),
            InputEvent::Focus => println!("Focus"),
            InputEvent::Blur => println!("Blur"),
        };
    }
}

impl Focusable for InputStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InputStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("input-story")
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Normal Input")
                    .max_w_md()
                    .child(TextInput::new(&self.input1).cleanable())
                    .child(TextInput::new(&self.input2)),
            )
            .child(
                section("Input State")
                    .max_w_md()
                    .child(TextInput::new(&self.disabled_input).disabled(true))
                    .child(TextInput::new(&self.mask_input).mask_toggle().cleanable()),
            )
            .child(
                section("Prefix and Suffix")
                    .max_w_md()
                    .child(
                        TextInput::new(&self.prefix_input1)
                            .cleanable()
                            .prefix(Icon::new(IconName::Search).small()),
                    )
                    .child(
                        TextInput::new(&self.both_input1)
                            .cleanable()
                            .prefix(div().child(Icon::new(IconName::Search).small()))
                            .suffix(Button::new("info").ghost().icon(IconName::Info).xsmall()),
                    )
                    .child(
                        TextInput::new(&self.suffix_input1)
                            .cleanable()
                            .suffix(Button::new("info").ghost().icon(IconName::Info).xsmall()),
                    ),
            )
            .child(
                section("Currency Input with thousands separator")
                    .max_w_md()
                    .child(TextInput::new(&self.currency_input))
                    .child(
                        div().child(format!("Value: {:?}", self.currency_input.read(cx).value())),
                    ),
            )
            .child(
                section("Input with mask pattern: (999)-999-9999")
                    .max_w_md()
                    .child(TextInput::new(&self.phone_input))
                    .child(
                        v_flex()
                            .child(format!("Value: {:?}", self.phone_input.read(cx).value()))
                            .child(format!(
                                "Unmask Value: {:?}",
                                self.phone_input.read(cx).unmask_value()
                            )),
                    ),
            )
            .child(
                section("Input with mask pattern: AAA-###-AAA")
                    .max_w_md()
                    .child(TextInput::new(&self.mask_input2))
                    .child(
                        v_flex()
                            .child(format!("Value: {:?}", self.mask_input2.read(cx).value()))
                            .child(format!(
                                "Unmask Value: {:?}",
                                self.mask_input2.read(cx).unmask_value()
                            )),
                    ),
            )
            .child(
                section("Input Size")
                    .max_w_md()
                    .child(TextInput::new(&self.large_input).large().cleanable())
                    .child(TextInput::new(&self.small_input).small().cleanable()),
            )
            .child(
                section("Cleanable and ESC to clean")
                    .max_w_md()
                    .child(TextInput::new(&self.input_esc).cleanable()),
            )
            .child(
                section("Focused Input")
                    .max_w_md()
                    .whitespace_normal()
                    .overflow_hidden()
                    .child(div().child(format!(
                        "Value: {:?}",
                        window.focused_input(cx).map(|input| input.read(cx).value())
                    ))),
            )
    }
}
