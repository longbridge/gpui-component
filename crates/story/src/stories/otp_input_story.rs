use gpui::{
    prelude::FluentBuilder as _, px, App, AppContext as _, Context, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, SharedString, Styled,
    Subscription, Window,
};
use gpui_component::{
    checkbox::Checkbox,
    h_flex,
    input::{InputEvent, OtpInput, OtpState},
    v_flex, Disableable as _, Sizable, StyledExt,
};

use crate::section;

pub fn init(_: &mut App) {}

pub struct OtpInputStory {
    otp_masked: bool,
    otp_state: Entity<OtpState>,
    otp_value: Option<SharedString>,
    otp_state_small: Entity<OtpState>,
    otp_state_large: Entity<OtpState>,
    otp_state_sized: Entity<OtpState>,
    otp_state_disabled: Entity<OtpState>,

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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl OtpInputStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));

        let _subscriptions = vec![
            cx.subscribe(&otp_state, |this, state, ev: &InputEvent, cx| match ev {
                InputEvent::Change => {
                    let text = state.read(cx).value();
                    this.otp_value = Some(text.clone());
                    cx.notify();
                }
                _ => {}
            }),
        ];

        Self {
            otp_masked: true,
            otp_state,
            otp_value: None,
            otp_state_small: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .default_value("123456")
                    .masked(true)
            }),
            otp_state_large: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .default_value("012345")
                    .masked(true)
            }),
            otp_state_sized: cx.new(|cx| {
                OtpState::new(4, window, cx)
                    .masked(true)
                    .default_value("654321")
            }),
            otp_state_disabled: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .masked(true)
                    .default_value("123456")
            }),
            _subscriptions,
        }
    }

    fn toggle_opt_masked(&mut self, _: &bool, window: &mut Window, cx: &mut Context<Self>) {
        self.otp_masked = !self.otp_masked;
        self.otp_state.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_small.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_large.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_sized.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
        self.otp_state_disabled.update(cx, |state, cx| {
            state.set_masked(self.otp_masked, window, cx)
        });
    }
}

impl Focusable for OtpInputStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.otp_state.focus_handle(cx)
    }
}

impl Render for OtpInputStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("otp-input-story")
            .size_full()
            .gap_5()
            .child(
                h_flex().items_center().child(
                    Checkbox::new("otp-mask")
                        .label("Masked")
                        .checked(self.otp_masked)
                        .on_click(cx.listener(Self::toggle_opt_masked)),
                ),
            )
            .child(
                section("Normal")
                    .v_flex()
                    .child(OtpInput::new(&self.otp_state))
                    .when_some(self.otp_value.clone(), |this, otp| {
                        this.child(format!("Your OTP: {}", otp))
                    }),
            )
            .child(section("Small").child(OtpInput::new(&self.otp_state_small).groups(1).small()))
            .child(section("Large").child(OtpInput::new(&self.otp_state_large).groups(3).large()))
            .child(
                section("With Size").child(
                    OtpInput::new(&self.otp_state_sized)
                        .groups(1)
                        .with_size(px(55.)),
                ),
            )
            .child(
                section("Disabled").child(OtpInput::new(&self.otp_state_disabled).disabled(true)),
            )
    }
}
