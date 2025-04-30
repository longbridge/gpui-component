use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, AppContext as _, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement as _, Render, SharedString, Styled as _, Subscription, Window,
};

use crate::{h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable, Size};

use super::{blink_cursor::BlinkCursor, InputEvent};

pub enum InputOptEvent {
    /// When all OTP input have filled, this event will be triggered.
    Change(SharedString),
}

/// A One Time Password (OTP) input element.
///
/// This can accept a fixed length number and can be masked.
///
/// Use case example:
///
/// - SMS OTP
/// - Authenticator OTP
pub struct OtpInput {
    focus_handle: FocusHandle,
    length: usize,
    number_of_groups: usize,
    masked: bool,
    value: SharedString,
    blink_cursor: Entity<BlinkCursor>,
    size: Size,
    _subscriptions: Vec<Subscription>,
}

impl OtpInput {
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
            focus_handle: focus_handle.clone(),
            length,
            number_of_groups: 2,
            value: SharedString::default(),
            masked: false,
            blink_cursor: blink_cursor.clone(),
            size: Size::Medium,
            _subscriptions,
        }
    }

    /// Set number of groups in the OTP Input.
    pub fn groups(mut self, n: usize) -> Self {
        self.number_of_groups = n;
        self
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

    pub fn focus(&self, window: &mut Window, _: &mut Context<Self>) {
        self.focus_handle.focus(window);
    }

    fn on_input_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
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
            cx.emit(InputEvent::Change(self.value.clone()));
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

impl Sizable for OtpInput {
    fn with_size(mut self, size: impl Into<crate::Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for OtpInput {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<InputEvent> for OtpInput {}

impl Render for OtpInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blink_show = self.blink_cursor.read(cx).visible();
        let is_focused = self.focus_handle.is_focused(window);

        let text_size = match self.size {
            Size::XSmall => px(14.),
            Size::Small => px(14.),
            Size::Medium => px(16.),
            Size::Large => px(18.),
            Size::Size(v) => v * 0.5,
        };

        let mut groups: Vec<Vec<AnyElement>> = Vec::with_capacity(self.number_of_groups);
        let mut group_ix = 0;
        let group_items_count = self.length / self.number_of_groups;
        for _ in 0..self.number_of_groups {
            groups.push(vec![]);
        }

        for i in 0..self.length {
            let c = self.value.chars().nth(i);
            if i % group_items_count == 0 && i != 0 {
                group_ix += 1;
            }

            let is_input_focused = i == self.value.chars().count() && is_focused;

            groups[group_ix].push(
                h_flex()
                    .id(("input-otp", i))
                    .border_1()
                    .border_color(cx.theme().input)
                    .bg(cx.theme().background)
                    .when(is_input_focused, |this| this.border_color(cx.theme().ring))
                    .when(cx.theme().shadow, |this| this.shadow_sm())
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
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_input_mouse_down))
                    .map(|this| match c {
                        Some(c) => {
                            if self.masked {
                                this.child(
                                    Icon::new(IconName::Asterisk)
                                        .text_color(cx.theme().secondary_foreground)
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
                                    .border_color(crate::blue_500()),
                            )
                        }),
                    })
                    .into_any_element(),
            );
        }

        v_flex()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
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
