use gpui::{
    AnyElement, App, AppContext as _, Context, Empty, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement as _,
    Render, RenderOnce, SharedString, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder, px,
};

use super::{InputEvent, blink_cursor::BlinkCursor};
use crate::{ActiveTheme, Disableable, Icon, IconName, Sizable, Size, h_flex, v_flex};

pub struct OtpState {
    focus_handle: FocusHandle,
    value: SharedString,
    blink_cursor: Entity<BlinkCursor>,
    masked: bool,
    length: usize,
    _subscriptions: Vec<Subscription>,
}

impl OtpState {
    /// Create a new [`OtpState`] with the specified length.
    pub fn new(length: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let blink_cursor = cx.new(|_| BlinkCursor::new());

        let _subscriptions = vec![
            // Observe the blink cursor to repaint the view when it changes.
            cx.observe(&blink_cursor, |_, _, cx| cx.notify()),
            // Blink the cursor when the window is active, pause when it's not.
            cx.observe_window_activation(window, |this, window, cx| {
                if window.is_window_active() {
                    let focus_handle = this.focus_handle.clone();
                    if focus_handle.is_focused(window) {
                        this.blink_cursor.update(cx, |blink_cursor, cx| {
                            blink_cursor.start(cx);
                        });
                    }
                }
            }),
            cx.on_focus(&focus_handle, window, Self::on_focus),
            cx.on_blur(&focus_handle, window, Self::on_blur),
        ];

        Self {
            length,
            focus_handle: focus_handle.clone(),
            value: SharedString::default(),
            blink_cursor: blink_cursor.clone(),
            masked: false,
            _subscriptions,
        }
    }

    /// Set default value of the OTP Input.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set value of the OTP Input.
    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value.into();
        cx.notify();
    }

    /// Return the value of the OTP Input.
    pub fn value(&self) -> &SharedString {
        &self.value
    }

    /// Set masked to true use masked input.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Set masked to true use masked input.
    pub fn set_masked(&mut self, masked: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.masked = masked;
        cx.notify();
    }

    /// Focus the OTP Input.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
    }

    fn on_input_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mut chars: Vec<char> = self.value.chars().collect();
        let ix = chars.len();

        let key = event.keystroke.key.as_str();

        match key {
            "backspace" => {
                if ix > 0 {
                    let ix = ix - 1;
                    chars.remove(ix);
                }

                window.prevent_default();
                cx.stop_propagation();
            }
            _ => {
                let c = key.chars().next().unwrap();
                if !matches!(c, '0'..='9') {
                    return;
                }
                if ix >= self.length {
                    return;
                }

                chars.push(c);

                window.prevent_default();
                cx.stop_propagation();
            }
        }

        self.pause_blink_cursor(cx);
        self.value = SharedString::from(chars.iter().collect::<String>());

        if self.value.chars().count() == self.length {
            cx.emit(InputEvent::Change);
        }
        cx.notify()
    }

    fn on_focus(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.start(cx);
        });
        cx.emit(InputEvent::Focus);
    }

    fn on_blur(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.stop(cx);
        });
        cx.emit(InputEvent::Blur);
    }

    fn pause_blink_cursor(&mut self, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.pause(cx);
        });
    }
}
impl Focusable for OtpState {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<InputEvent> for OtpState {}
impl Render for OtpState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// A One Time Password (OTP) input element.
///
/// This can accept a fixed length number and can be masked.
///
/// Use case example:
///
/// - SMS OTP
/// - Authenticator OTP
#[derive(IntoElement)]
pub struct OtpInput {
    state: Entity<OtpState>,
    number_of_groups: usize,
    size: Size,
    disabled: bool,
}

impl OtpInput {
    /// Create a new [`OtpInput`] element bind to the [`OtpState`].
    pub fn new(state: &Entity<OtpState>) -> Self {
        Self {
            state: state.clone(),
            number_of_groups: 2,
            size: Size::Medium,
            disabled: false,
        }
    }

    /// Set number of groups in the OTP Input.
    pub fn groups(mut self, n: usize) -> Self {
        self.number_of_groups = n;
        self
    }
}
impl Disableable for OtpInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
impl Sizable for OtpInput {
    fn with_size(mut self, size: impl Into<crate::Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl RenderOnce for OtpInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let blink_show = state.blink_cursor.read(cx).visible();
        let is_focused = state.focus_handle.is_focused(window);

        let text_size = match self.size {
            Size::XSmall => px(14.),
            Size::Small => px(14.),
            Size::Medium => px(16.),
            Size::Large => px(18.),
            Size::Size(v) => v * 0.5,
        };

        let cursor_ix = state
            .value
            .chars()
            .count()
            .min(state.length.saturating_sub(1));
        let mut groups: Vec<Vec<AnyElement>> = Vec::with_capacity(self.number_of_groups);
        let mut group_ix = 0;
        let group_items_count = state.length / self.number_of_groups;
        for _ in 0..self.number_of_groups {
            groups.push(vec![]);
        }

        for ix in 0..state.length {
            let c = state.value.chars().nth(ix);
            if ix % group_items_count == 0 && ix != 0 {
                group_ix += 1;
            }

            let is_input_focused = ix == cursor_ix && is_focused;

            groups[group_ix].push(
                h_flex()
                    .id(ix)
                    .border_1()
                    .border_color(cx.theme().input)
                    .bg(cx.theme().background)
                    .when(self.disabled, |this| {
                        this.bg(cx.theme().muted)
                            .text_color(cx.theme().muted_foreground)
                    })
                    .when(is_input_focused, |this| this.border_color(cx.theme().ring))
                    .when(cx.theme().shadow, |this| this.shadow_xs())
                    .items_center()
                    .justify_center()
                    .rounded(cx.theme().radius)
                    .text_size(text_size)
                    .map(|this| match self.size {
                        Size::XSmall => this.w_6().h_6(),
                        Size::Small => this.w_6().h_6(),
                        Size::Medium => this.w_8().h_8(),
                        Size::Large => this.w_11().h_11(),
                        Size::Size(px) => this.w(px).h(px),
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        window.listener_for(&self.state, OtpState::on_input_mouse_down),
                    )
                    .map(|this| match c {
                        Some(c) => {
                            if state.masked {
                                this.child(
                                    Icon::new(IconName::Asterisk)
                                        .text_color(cx.theme().secondary_foreground)
                                        .when(self.disabled, |this| {
                                            this.text_color(cx.theme().muted_foreground)
                                        })
                                        .with_size(text_size),
                                )
                            } else {
                                this.child(c.to_string())
                            }
                        }
                        None => this.when(is_input_focused && blink_show, |this| {
                            this.child(
                                div()
                                    .h_4()
                                    .w_0()
                                    .border_l_3()
                                    .border_color(cx.theme().caret),
                            )
                        }),
                    })
                    .into_any_element(),
            );
        }

        v_flex()
            .id(("otp-input", self.state.entity_id()))
            .track_focus(&self.state.read(cx).focus_handle)
            .when(!self.disabled, |this| {
                this.on_key_down(window.listener_for(&self.state, OtpState::on_key_down))
            })
            .items_center()
            .child(
                h_flex().items_center().gap_5().children(
                    groups
                        .into_iter()
                        .map(|inputs| h_flex().items_center().gap_1().children(inputs)),
                ),
            )
    }
}
