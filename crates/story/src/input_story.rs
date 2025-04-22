use gpui::{
    actions, div, App, AppContext as _, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ParentElement as _, Render, Styled, Subscription,
    Window,
};

use crate::section;
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{InputEvent, TextInput},
    v_flex, ContextModal, FocusableCycle, Icon, IconName, Root, Sizable,
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
    input_esc: Entity<TextInput>,
    mask_input: Entity<TextInput>,
    disabled_input: Entity<TextInput>,
    prefix_input1: Entity<TextInput>,
    suffix_input1: Entity<TextInput>,
    both_input1: Entity<TextInput>,
    large_input: Entity<TextInput>,
    small_input: Entity<TextInput>,

    _subscriptions: Vec<Subscription>,
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
            let mut input = TextInput::new(window, cx).cleanable().h_full();
            input.set_text(
                "Hello 世界，this is GPUI component, this is a long text.",
                window,
                cx,
            );
            input
        });

        let input2 = cx.new(|cx| TextInput::new(window, cx).placeholder("Enter text here..."));
        let input_esc = cx.new(|cx| {
            TextInput::new(window, cx)
                .placeholder("Enter text and clear it by pressing ESC")
                .cleanable()
                .clean_on_escape()
        });

        let mask_input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx)
                .masked(true)
                .mask_toggle()
                .cleanable();
            input.set_text("this-is-password", window, cx);
            input
        });

        let prefix_input1 = cx.new(|cx| {
            TextInput::new(window, cx)
                .prefix(|_, _| div().child(Icon::new(IconName::Search).small()).ml_3())
                .placeholder("Search some thing...")
                .cleanable()
        });
        let suffix_input1 = cx.new(|cx| {
            TextInput::new(window, cx)
                .suffix(|_, _| {
                    Button::new("info")
                        .ghost()
                        .icon(IconName::Info)
                        .xsmall()
                        .mr_3()
                })
                .placeholder("This input only support [a-zA-Z0-9] characters.")
                .pattern(regex::Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
                .cleanable()
        });
        let both_input1 = cx.new(|cx| {
            TextInput::new(window, cx)
                .prefix(|_, _| div().child(Icon::new(IconName::Search).small()).ml_3())
                .suffix(|_, _| {
                    Button::new("info")
                        .ghost()
                        .icon(IconName::Info)
                        .xsmall()
                        .mr_3()
                })
                .cleanable()
                .placeholder("This input have prefix and suffix.")
        });

        let _subscriptions = vec![
            cx.subscribe_in(&input1, window, Self::on_input_event),
            cx.subscribe_in(&input2, window, Self::on_input_event),
        ];

        Self {
            input1,
            input2,
            input_esc,
            mask_input,
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
            _subscriptions,
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
            InputEvent::PressEnter { secondary } => println!("PressEnter secondary: {}", secondary),
            InputEvent::Focus => println!("Focus"),
            InputEvent::Blur => println!("Blur"),
        };
    }
}

impl FocusableCycle for InputStory {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        [
            self.input1.focus_handle(cx),
            self.input2.focus_handle(cx),
            self.input_esc.focus_handle(cx),
            self.disabled_input.focus_handle(cx),
            self.mask_input.focus_handle(cx),
            self.prefix_input1.focus_handle(cx),
            self.both_input1.focus_handle(cx),
            self.suffix_input1.focus_handle(cx),
            self.large_input.focus_handle(cx),
            self.small_input.focus_handle(cx),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("input-story")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .size_full()
            .justify_start()
            .gap_3()
            .child(
                section("Normal Input")
                    .max_w_md()
                    .child(self.input1.clone())
                    .child(self.input2.clone()),
            )
            .child(
                section("Input State")
                    .max_w_md()
                    .child(self.disabled_input.clone())
                    .child(self.mask_input.clone()),
            )
            .child(
                section("Prefix and Suffix")
                    .max_w_md()
                    .child(self.prefix_input1.clone())
                    .child(self.both_input1.clone())
                    .child(self.suffix_input1.clone()),
            )
            .child(
                section("Input Size")
                    .max_w_md()
                    .child(self.large_input.clone())
                    .child(self.small_input.clone()),
            )
            .child(
                section("Cleanable and ESC to clean")
                    .max_w_md()
                    .child(self.input_esc.clone()),
            )
            .child(
                section("Focused Input")
                    .max_w_md()
                    .whitespace_normal()
                    .overflow_hidden()
                    .child(div().child(format!(
                        "Value: {:?}",
                        window.focused_input(cx).map(|input| input.read(cx).text())
                    ))),
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
