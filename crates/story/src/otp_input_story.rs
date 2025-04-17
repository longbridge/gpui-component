use gpui::{
    actions, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, ParentElement as _, Render,
    SharedString, Styled, Subscription, Window,
};
use gpui_component::{
    checkbox::Checkbox,
    h_flex,
    input::{InputEvent, OtpInput},
    v_flex, FocusableCycle, Sizable, StyledExt,
};

use crate::section;
actions!(input_story, [Tab, TabPrev]);

const CONTEXT: &str = "OtpInputStory";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
    ])
}

pub struct OtpInputStory {
    otp_masked: bool,
    otp_input: Entity<OtpInput>,
    otp_value: Option<SharedString>,
    otp_input_small: Entity<OtpInput>,
    otp_input_large: Entity<OtpInput>,
    opt_input_sized: Entity<OtpInput>,

    _subscriptions: Vec<Subscription>,
}

impl super::Story for OtpInputStory {
    fn title() -> &'static str {
        "OtpInput"
    }

    fn description() -> &'static str {
        "OTP Input uses to one-time password (OTP) input field or number password input field."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl OtpInputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_input = cx.new(|cx| OtpInput::new(6, window, cx).masked(true));

        let _subscriptions =
            vec![
                cx.subscribe(&otp_input, |this, _, ev: &InputEvent, cx| match ev {
                    InputEvent::Change(text) => {
                        this.otp_value = Some(text.clone());
                        cx.notify();
                    }
                    _ => {}
                }),
            ];

        Self {
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
            _subscriptions,
        }
    }

    fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(true, window, cx);
    }

    fn tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_focus(false, window, cx);
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

impl FocusableCycle for OtpInputStory {
    fn cycle_focus_handles(&self, _: &mut Window, cx: &mut App) -> Vec<FocusHandle> {
        [self.otp_input.focus_handle(cx)].to_vec()
    }
}
impl Focusable for OtpInputStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.otp_input.focus_handle(cx)
    }
}

impl Render for OtpInputStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context(CONTEXT)
            .id("otp-input-story")
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::tab_prev))
            .size_full()
            .gap_5()
            .child(
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
            )
            .child(
                section("Normal")
                    .v_flex()
                    .child(self.otp_input.clone())
                    .when_some(self.otp_value.clone(), |this, otp| {
                        this.child(format!("Your OTP: {}", otp))
                    }),
            )
            .child(section("Small").child(self.otp_input_small.clone()))
            .child(section("Large").child(self.otp_input_large.clone()))
            .child(section("With Size").child(self.opt_input_sized.clone()))
    }
}
