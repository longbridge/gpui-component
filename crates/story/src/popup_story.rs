use gpui::{
    actions, div, impl_internal_actions, px, AppContext, Corner, DismissEvent, Element,
    EventEmitter, FocusHandle, FocusableView, InteractiveElement, IntoElement, KeyBinding,
    MouseButton, ParentElement as _, Render, SharedString, Styled as _, View, ViewContext,
    VisualContext, WindowContext,
};
use serde::Deserialize;
use ui::{
    button::{Button, ButtonVariants as _},
    context_menu::ContextMenuExt,
    divider::Divider,
    h_flex,
    input::TextInput,
    popover::{Popover, PopoverContent},
    popup_menu::PopupMenuExt,
    switch::Switch,
    v_flex, ActiveTheme as _, ContextModal, IconName, Sizable,
};

#[derive(Clone, PartialEq, Deserialize)]
struct Info(usize);

actions!(
    popover_story,
    [Copy, Paste, Cut, SearchAll, ToggleWindowMode]
);
impl_internal_actions!(popover_story, [Info]);

pub fn init(cx: &mut AppContext) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", SearchAll, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", SearchAll, None),
    ])
}

struct Form {
    input1: View<TextInput>,
}

impl Form {
    fn new(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| Self {
            input1: cx.new_view(TextInput::new),
        })
    }
}

impl FocusableView for Form {
    fn focus_handle(&self, cx: &AppContext) -> FocusHandle {
        self.input1.focus_handle(cx)
    }
}

impl EventEmitter<DismissEvent> for Form {}

impl Render for Form {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .p_4()
            .size_full()
            .child("This is a form container.")
            .child(self.input1.clone())
            .child(
                Button::new("submit")
                    .label("Submit")
                    .primary()
                    .on_click(cx.listener(|_, _, cx| cx.emit(DismissEvent))),
            )
    }
}

pub struct PopupStory {
    focus_handle: FocusHandle,
    form: View<Form>,
    message: String,
    window_mode: bool,
}

impl super::Story for PopupStory {
    fn title() -> &'static str {
        "Popup"
    }

    fn description() -> &'static str {
        "A popup displays content on top of the main page."
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl PopupStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    fn new(cx: &mut ViewContext<Self>) -> Self {
        let form = Form::new(cx);
        Self {
            form,
            focus_handle: cx.focus_handle(),
            message: "".to_string(),
            window_mode: false,
        }
    }

    fn on_copy(&mut self, _: &Copy, cx: &mut ViewContext<Self>) {
        self.message = "You have clicked copy".to_string();
        cx.notify()
    }
    fn on_cut(&mut self, _: &Cut, cx: &mut ViewContext<Self>) {
        self.message = "You have clicked cut".to_string();
        cx.notify()
    }
    fn on_paste(&mut self, _: &Paste, cx: &mut ViewContext<Self>) {
        self.message = "You have clicked paste".to_string();
        cx.notify()
    }
    fn on_search_all(&mut self, _: &SearchAll, cx: &mut ViewContext<Self>) {
        self.message = "You have clicked search all".to_string();
        cx.notify()
    }
    fn on_toggle_window_mode(&mut self, _: &ToggleWindowMode, cx: &mut ViewContext<Self>) {
        self.window_mode = !self.window_mode;
        cx.notify()
    }
    fn on_action_info(&mut self, info: &Info, cx: &mut ViewContext<Self>) {
        self.message = format!("You have clicked info: {}", info.0);
        cx.notify()
    }
}

impl FocusableView for PopupStory {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopupStory {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let form = self.form.clone();
        let window_mode = self.window_mode;

        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_toggle_window_mode))
            .on_action(cx.listener(Self::on_action_info))
            .p_4()
            .mb_5()
            .size_full()
            .min_h(px(400.))
            .context_menu({
                move |this, cx| {
                    this.separator()
                        .menu("Cut", Box::new(Cut))
                        .menu("Copy", Box::new(Copy))
                        .menu("Paste", Box::new(Paste))
                        .separator()
                        .separator()
                        .submenu("Settings", cx, move |menu, _| {
                            menu.menu_with_check(
                                "Toggle Window Mode",
                                window_mode,
                                Box::new(ToggleWindowMode),
                            )
                            .separator()
                            .menu("Info 0", Box::new(Info(0)))
                            .menu("Item 1", Box::new(Info(1)))
                            .menu("Item 2", Box::new(Info(2)))
                        })
                        .separator()
                        .menu("Search All", Box::new(SearchAll))
                        .separator()
                }
            })
            .gap_6()
            .child(
                Switch::new("switch-window-mode")
                    .checked(window_mode)
                    .label("Use Window Popover")
                    .on_click(cx.listener(|this, checked, _| {
                        this.window_mode = *checked;
                    })),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_4().child(
                            Popover::new("info-top-left")
                                .trigger(Button::new("info-top-left").label("Top Left"))
                                .content(|cx| {
                                    cx.new_view(|cx| {
                                        PopoverContent::new(cx, |_| {
                                            v_flex()
                                                .gap_4()
                                                .child("Hello, this is a Popover.")
                                                .w(px(400.))
                                                .child(Divider::horizontal())
                                                .child(
                                                    Button::new("info1")
                                                        .label("Yes")
                                                        .w(px(80.))
                                                        .small(),
                                                )
                                                .into_any()
                                        })
                                        .max_w(px(600.))
                                    })
                                }),
                        ),
                    )
                    .child(
                        Popover::new("info-top-right")
                            .anchor(Corner::TopRight)
                            .trigger(Button::new("info-top-right").label("Top Right"))
                            .content(|cx| {
                                cx.new_view(|cx| {
                                    PopoverContent::new(cx, |_| {
                                        v_flex()
                                            .gap_4()
                                            .w_96()
                                            .child("Hello, this is a Popover on the Top Right.")
                                            .child(Divider::horizontal())
                                            .child(
                                                Button::new("info1")
                                                    .label("Yes")
                                                    .w(px(80.))
                                                    .small(),
                                            )
                                            .into_any()
                                    })
                                })
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("popup-menu-1")
                            .icon(IconName::Ellipsis)
                            .popup_menu(move |this, cx| {
                                this.menu("Copy", Box::new(Copy))
                                    .menu("Cut", Box::new(Cut))
                                    .menu("Paste", Box::new(Paste))
                                    .separator()
                                    .menu_with_icon("Search", IconName::Search, Box::new(SearchAll))
                                    .separator()
                                    .menu_with_check(
                                        "Window Mode",
                                        window_mode,
                                        Box::new(ToggleWindowMode),
                                    )
                                    .separator()
                                    .menu_with_element(
                                        |cx| {
                                            v_flex().gap_1().child("Custom Element").child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("THis is sub-title"),
                                            )
                                        },
                                        Box::new(Info(0)),
                                    )
                                    .separator()
                                    .submenu("Links", cx, |menu, _| {
                                        menu.link_with_icon(
                                            "GitHub Repository",
                                            IconName::GitHub,
                                            "https://github.com/longbridge/gpui-component",
                                        )
                                        .separator()
                                        .link("GPUI", "https://gpui.rs")
                                        .link("Zed", "https://zed.dev")
                                    })
                            }),
                    )
                    .child(
                        Button::new("popup-menu-11112")
                            .label("Scrollable Menu")
                            .popup_menu_with_anchor(Corner::TopRight, move |this, _| {
                                let mut this = this.scrollable();
                                for i in 0..100 {
                                    this = this.menu(
                                        SharedString::from(format!("Item {}", i)),
                                        Box::new(Info(i)),
                                    )
                                }
                                this
                            }),
                    )
                    .child(self.message.clone()),
            )
            .child("Right click to open ContextMenu")
            .child(
                div().absolute().bottom_4().left_0().w_full().h_10().child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            Popover::new("info-bottom-left")
                                .anchor(Corner::BottomLeft)
                                .trigger(Button::new("pop").label("Popup with Form").w(px(300.)))
                                .content(move |_| form.clone()),
                        )
                        .child(
                            Popover::new("info-bottom-right")
                                .anchor(Corner::BottomRight)
                                .mouse_button(MouseButton::Right)
                                .trigger(Button::new("pop").label("Mouse Right Click").w(px(300.)))
                                .content(|cx| {
                                    cx.new_view(|cx| {
                                        PopoverContent::new(cx, |cx| {
                                            v_flex()
                                                .gap_2()
                                                .child(
                                                    "Hello, this is a Popover on the Bottom Right.",
                                                )
                                                .child(Divider::horizontal())
                                                .child(
                                                    h_flex()
                                                        .gap_2()
                                                        .child(
                                                            Button::new("info1")
                                                                .label("Ok")
                                                                .w(px(80.))
                                                                .small()
                                                                .on_click(cx.listener(
                                                                    |_, _, cx| {
                                                                        cx.push_notification(
                                                                            "You have clicked Ok.",
                                                                        );
                                                                        cx.emit(DismissEvent);
                                                                    },
                                                                )),
                                                        )
                                                        .child(
                                                            Button::new("close")
                                                                .label("Cancel")
                                                                .small()
                                                                .on_click(cx.listener(
                                                                    |_, _, cx| {
                                                                        cx.emit(DismissEvent);
                                                                    },
                                                                )),
                                                        ),
                                                )
                                                .into_any()
                                        })
                                    })
                                }),
                        ),
                ),
            )
    }
}
