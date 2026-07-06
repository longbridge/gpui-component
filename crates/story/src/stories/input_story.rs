use gpui::{
    App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement, IntoElement,
    ParentElement as _, Render, Role, Styled, Subscription, Window, div,
};

use crate::section;
use gpui_component::{button::*, input::*, label::Label, *};

const CODE_EXAMPLE: &str = r#"{"single_line":"code editor"}"#;

pub fn init(_: &mut App) {}

pub struct InputStory {
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    input_esc: Entity<InputState>,
    input_text_centered: Entity<InputState>,
    input_text_right: Entity<InputState>,
    mask_input: Entity<InputState>,
    disabled_input: Entity<InputState>,
    prefix_input1: Entity<InputState>,
    suffix_input1: Entity<InputState>,
    both_input1: Entity<InputState>,
    complete_input: Entity<InputState>,
    complete_disabled_input: Entity<InputState>,
    large_input: Entity<InputState>,
    small_input: Entity<InputState>,
    phone_input: Entity<InputState>,
    mask_input2: Entity<InputState>,
    currency_input: Entity<InputState>,
    custom_input: Entity<InputState>,
    custom_menu_input: Entity<InputState>,
    code_input: Entity<InputState>,
    color_input: Entity<InputState>,
    content_type_inputs: Vec<ContentTypeInput>,

    _subscriptions: Vec<Subscription>,
}

struct ContentTypeInput {
    label: &'static str,
    content_type: InputContentType,
    input: Entity<InputState>,
    mask_toggle: bool,
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
                .placeholder("Enter your password...")
                .default_value("this-is-password-中文🚀🎉")
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
        let complete_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search account...")
                .default_value("jane.doe@example.com")
        });
        let complete_disabled_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search account...")
                .default_value("disabled.account@example.com")
        });

        let phone_input = cx.new(|cx| InputState::new(window, cx).mask_pattern("(999)-999-9999"));
        let mask_input2 = cx.new(|cx| InputState::new(window, cx).mask_pattern("AAA-###-AAA"));
        let currency_input = cx.new(|cx| {
            InputState::new(window, cx).mask_pattern(MaskPattern::Number {
                separator: Some(','),
                fraction: Some(3),
            })
        });
        let custom_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Custom Input use monospace, 0123456789.")
                .context_menu(false)
        });

        let custom_menu_input = cx
            .new(|cx| InputState::new(window, cx).placeholder("Input with custom context menu..."));

        let color_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type something...")
                .default_value("Custom text color input")
        });

        let code_input = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .multi_line(false)
                .show_whitespaces(true)
                .default_value(CODE_EXAMPLE)
        });

        let input_text_centered = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter text to test center layout...")
                .default_value("Centered Text")
        });

        let input_text_right = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter text to test right layout...")
                .default_value("Right Aligned Text")
        });

        let content_type_inputs = vec![
            Self::new_content_type_input(
                window,
                cx,
                "Name",
                InputContentType::Name,
                "Full name",
                "Jane Doe",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Username",
                InputContentType::Username,
                "Username",
                "jane.doe",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Password",
                InputContentType::Password,
                "Current password",
                "current-password",
                true,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "New password",
                InputContentType::NewPassword,
                "New password",
                "new-password",
                true,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "One-time code",
                InputContentType::OneTimeCode,
                "123456",
                "123456",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Email",
                InputContentType::EmailAddress,
                "Email address",
                "jane.doe@example.com",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Telephone",
                InputContentType::TelephoneNumber,
                "Telephone number",
                "+1 415 555 0198",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "URL",
                InputContentType::Url,
                "Website URL",
                "https://example.com",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Credit card number",
                InputContentType::CreditCardNumber,
                "Card number",
                "4242 4242 4242 4242",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Credit card expiration",
                InputContentType::CreditCardExpiration,
                "MM/YY",
                "12/28",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Credit card security code",
                InputContentType::CreditCardSecurityCode,
                "CVC",
                "123",
                true,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Postal code",
                InputContentType::PostalCode,
                "Postal code",
                "94107",
                false,
            ),
        ];

        let _subscriptions = vec![
            cx.subscribe_in(&input1, window, Self::on_input_event),
            cx.subscribe_in(&input2, window, Self::on_input_event),
            cx.subscribe_in(&phone_input, window, Self::on_input_event),
        ];

        Self {
            input1,
            input2,
            input_esc,
            mask_input,
            disabled_input: cx
                .new(|cx| InputState::new(window, cx).default_value("This is disabled input")),
            large_input: cx.new(|cx| InputState::new(window, cx).placeholder("Large input")),
            small_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .validate(|s, _| s.parse::<f32>().is_ok())
                    .placeholder("validate to limit float number.")
            }),
            prefix_input1,
            suffix_input1,
            both_input1,
            complete_input,
            complete_disabled_input,
            phone_input,
            mask_input2,
            currency_input,
            custom_input,
            custom_menu_input,
            code_input,
            color_input,
            input_text_centered,
            input_text_right,
            content_type_inputs,
            _subscriptions,
        }
    }

    fn new_content_type_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        label: &'static str,
        content_type: InputContentType,
        placeholder: &'static str,
        default_value: &'static str,
        masked: bool,
    ) -> ContentTypeInput {
        let input = cx.new(|cx| {
            let state = InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(default_value);

            if masked { state.masked(true) } else { state }
        });

        ContentTypeInput {
            label,
            content_type,
            input,
            mask_toggle: masked,
        }
    }

    fn render_content_type_input(item: &ContentTypeInput) -> impl IntoElement {
        let input = Input::new(&item.input)
            .content_type(item.content_type)
            .flex_1();
        let input = if item.mask_toggle {
            input.mask_toggle()
        } else {
            input
        };

        h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .child(
                Label::new(item.label)
                    .w_48()
                    .flex_shrink_0()
                    .text_sm()
                    .whitespace_nowrap(),
            )
            .child(input)
    }

    fn on_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let text = state.read(cx).value();
                if state == &self.input2 {
                    println!("Set disabled value: {}", text);
                    self.disabled_input.update(cx, |this, cx| {
                        this.set_value(text, window, cx);
                    })
                } else {
                    println!("Change: {}", text)
                }
            }
            InputEvent::PressEnter { secondary, shift } => {
                println!("PressEnter secondary: {}, shift: {}", secondary, shift)
            }
            InputEvent::Focus => println!("Focus"),
            InputEvent::Blur => println!("Blur"),
        };
    }

    fn on_click_reset(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.code_input.update(cx, |input_state, cx| {
            input_state.set_value(CODE_EXAMPLE, window, cx);
        });
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
                    .child(Input::new(&self.input1).cleanable(true))
                    .child(Input::new(&self.input2).role(Role::EmailInput)),
            )
            .child(
                section("Input State")
                    .max_w_md()
                    .child(Input::new(&self.disabled_input).disabled(true))
                    .child(
                        Input::new(&self.mask_input)
                            .content_type(InputContentType::Password)
                            .mask_toggle()
                            .cleanable(true),
                    ),
            )
            .child(
                section("Content Type").max_w_lg().children(
                    self.content_type_inputs
                        .iter()
                        .map(Self::render_content_type_input),
                ),
            )
            .child(
                section("Text Align").max_w_lg().child(
                    h_flex()
                        .w_full()
                        .gap_4()
                        .flex_wrap()
                        .child(Input::new(&self.input_text_centered).text_center().flex_1())
                        .child(Input::new(&self.input_text_right).text_right().flex_1()),
                ),
            )
            .child(
                section("Prefix and Suffix")
                    .max_w_md()
                    .child(
                        Input::new(&self.prefix_input1)
                            .cleanable(true)
                            .prefix(Icon::new(IconName::Search).small()),
                    )
                    .child(
                        Input::new(&self.both_input1)
                            .cleanable(true)
                            .prefix(div().child(Icon::new(IconName::Search).small()))
                            .suffix(Button::new("info").ghost().icon(IconName::Info).xsmall()),
                    )
                    .child(
                        Input::new(&self.suffix_input1)
                            .cleanable(true)
                            .suffix(Button::new("info").ghost().icon(IconName::Info).xsmall()),
                    ),
            )
            .child(
                section("Complete Input")
                    .max_w_md()
                    .child(
                        Input::new(&self.complete_input)
                            .cleanable(true)
                            .prefix(Icon::new(IconName::Search).small())
                            .suffix(
                                Button::new("complete-input-info")
                                    .ghost()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    )
                    .child(
                        Input::new(&self.complete_disabled_input)
                            .cleanable(true)
                            .disabled(true)
                            .prefix(Icon::new(IconName::Search).small())
                            .suffix(
                                Button::new("complete-disabled-input-info")
                                    .ghost()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    ),
            )
            .child(
                section("Currency Input with thousands separator")
                    .max_w_md()
                    .child(Input::new(&self.currency_input))
                    .child(
                        div().child(format!("Value: {:?}", self.currency_input.read(cx).value())),
                    ),
            )
            .child(
                section("Input with mask pattern: (999)-999-9999")
                    .max_w_md()
                    .child(Input::new(&self.phone_input))
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
                    .child(Input::new(&self.mask_input2))
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
                    .child(Input::new(&self.large_input).large())
                    .child(Input::new(&self.small_input).small()),
            )
            .child(
                section("Cleanable and ESC to clean")
                    .max_w_md()
                    .child(Input::new(&self.input_esc).cleanable(true)),
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
            .child(
                section("Custom Appearance").max_w_md().child(
                    div()
                        .border_b_2()
                        .px_6()
                        .py_3()
                        .font_family(cx.theme().mono_font_family.clone())
                        .border_color(cx.theme().border)
                        .bg(cx.theme().secondary)
                        .text_color(cx.theme().secondary_foreground)
                        .w_full()
                        .child(Input::new(&self.custom_input).appearance(false)),
                ),
            )
            .child(section("Custom Context Menu").max_w_md().child(
                Input::new(&self.custom_menu_input).context_menu(|menu, _, _| {
                    menu.menu("Custom Action", Box::new(input::SelectAll))
                        .separator()
                        .menu("Copy", Box::new(input::Copy))
                        .menu("Paste", Box::new(input::Paste))
                }),
            ))
            .child(
                section("Custom Text Color")
                    .max_w_md()
                    .child(Input::new(&self.color_input).text_color(cx.theme().info)),
            )
            .child(
                section("Single line code editor").max_w_md().child(
                    Input::new(&self.code_input).suffix(
                        Button::new("code-reset")
                            .ghost()
                            .label("Reset")
                            .xsmall()
                            .on_click(cx.listener(Self::on_click_reset)),
                    ),
                ),
            )
    }
}
