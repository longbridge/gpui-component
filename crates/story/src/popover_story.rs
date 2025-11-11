use gpui::{
    Action, App, AppContext, Context, Corner, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement as _,
    Render, Styled as _, Window, actions, div, px,
};
use gpui_component::{
    Sizable, WindowExt,
    button::{Button, ButtonVariants as _},
    divider::Divider,
    h_flex,
    input::{Input, InputState},
    popover::Popover,
    v_flex,
};
use serde::Deserialize;

use crate::section;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = popover_story, no_json)]
struct Info(usize);

actions!(popover_story, [Copy, Paste, Cut, SearchAll, ToggleCheck]);
const CONTEXT: &str = "popover-story";
pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, Some(CONTEXT)),
    ])
}

struct Form {
    input1: Entity<InputState>,
}

impl Form {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            input1: cx.new(|cx| InputState::new(window, cx)),
        })
    }
}

impl Focusable for Form {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input1.focus_handle(cx)
    }
}

impl EventEmitter<DismissEvent> for Form {}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .p_4()
            .size_full()
            .child("This is a form container.")
            .child(Input::new(&self.input1))
            .child(
                Button::new("submit")
                    .label("Submit")
                    .primary()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(DismissEvent))),
            )
    }
}

pub struct PopoverStory {
    focus_handle: FocusHandle,
    form: Entity<Form>,
    checked: bool,
    message: String,
}

impl super::Story for PopoverStory {
    fn title() -> &'static str {
        "Popover"
    }

    fn description() -> &'static str {
        "A popup displays content on top of the main page."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl PopoverStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let form = Form::new(window, cx);

        cx.focus_self(window);

        Self {
            form,
            checked: true,
            focus_handle: cx.focus_handle(),
            message: "".to_string(),
        }
    }

    fn on_copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked copy".to_string();
        cx.notify()
    }

    fn on_cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked cut".to_string();
        cx.notify()
    }

    fn on_paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked paste".to_string();
        cx.notify()
    }

    fn on_search_all(&mut self, _: &SearchAll, _: &mut Window, cx: &mut Context<Self>) {
        self.message = "You have clicked search all".to_string();
        cx.notify()
    }

    fn on_action_info(&mut self, info: &Info, _: &mut Window, cx: &mut Context<Self>) {
        self.message = format!("You have clicked info: {}", info.0);
        cx.notify()
    }

    fn on_action_toggle_check(&mut self, _: &ToggleCheck, _: &mut Window, cx: &mut Context<Self>) {
        self.checked = !self.checked;
        self.message = format!("You have clicked toggle check: {}", self.checked);
        cx.notify()
    }
}

impl Focusable for PopoverStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.clone();

        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_action_info))
            .on_action(cx.listener(Self::on_action_toggle_check))
            .size_full()
            .min_h(px(400.))
            .gap_6()
            .child(
                section("Basic Popover").child(
                    Popover::new("popover-0")
                        .max_w(px(600.))
                        .trigger(Button::new("btn").outline().label("Popover"))
                        .gap_4()
                        .w(px(400.))
                        .child("Hello, this is a Popover.")
                        .child(Divider::horizontal())
                        .child(
                            "You can put any content here, including text, buttons, forms, and more.",
                        )
                )
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_4().child(
                            Popover::new("info-top-left")
                                .max_w(px(600.))
                                .trigger(Button::new("info-top-left").outline().label("Top Left"))
                                .content(|_, _, cx| {
                                    v_flex()
                                        .gap_4()
                                        .child("Hello, this is a Popover.")
                                        .w(px(400.))
                                        .child(Divider::horizontal())
                                        .child(
                                            Button::new("info1")
                                                .primary()
                                                .label("Ok")
                                                .w(px(80.))
                                                .small()
                                                .on_click(cx.listener(|_, _, _, cx| {
                                                    cx.emit(DismissEvent);
                                                }))
                                        )
                                }),
                        ),
                    )
                    .child(
                        Popover::new("info-top-right")
                            .anchor(Corner::TopRight)
                            .trigger(Button::new("info-top-right").outline().label("Top Right"))
                            .content(|_, _, cx| {
                                v_flex()
                                    .gap_4()
                                    .w_96()
                                    .child("Hello, this is a Popover on the Top Right.")
                                    .child(Divider::horizontal())
                                    .child(
                                        Button::new("info1")
                                            .primary()
                                            .label("Ok")
                                            .w(px(80.))
                                            .small()
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                cx.emit(DismissEvent);
                                            }))
                                    )
                            }),
                    ),
            )
            .child(
                div().absolute().bottom_4().left_0().w_full().h_10().child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            Popover::new("info-bottom-left")
                                .p_0()
                                .anchor(Corner::BottomLeft)
                                .trigger(
                                    Button::new("pop")
                                        .outline()
                                        .label("Popup with Form")
                                        .w(px(300.)),
                                )
                                .track_focus(&form.focus_handle(cx))
                                .child(form.clone()),
                        )
                        .child(
                            Popover::new("info-bottom-right")
                                .anchor(Corner::BottomRight)
                                .mouse_button(MouseButton::Right)
                                .trigger(
                                    Button::new("pop")
                                        .outline()
                                        .label("Mouse Right Click")
                                        .w(px(300.)),
                                )
                                .content(|_, _, cx| {
                                    v_flex()
                                        .gap_4()
                                        .child("Hello, this is a Popover on the Bottom Right.")
                                        .child(Divider::horizontal())
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .justify_end()
                                                .child(
                                                    Button::new("info1")
                                                        .primary()
                                                        .label("Ok")
                                                        .w(px(80.))
                                                        .on_click(cx.listener(
                                                            |_, _, window, cx| {
                                                                window.push_notification(
                                                                    "You have clicked Ok via DismissEvent.",
                                                                    cx,
                                                                );
                                                                cx.emit(DismissEvent);
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new("close")
                                                        .label("Cancel")
                                                        .outline()
                                                        .on_click(cx.listener(
                                                            |state, _, window, cx| {
                                                                window.push_notification(
                                                                    "You have clicked Cancel.",
                                                                    cx,
                                                                );
                                                                state.dismiss(window, cx);
                                                            },
                                                        )),
                                                ),
                                        )
                                }),
                        ),
                ),
            )
    }
}
