use gpui::{
    actions, div, impl_internal_actions, px, App, AppContext, Context, Corner, DismissEvent,
    Element, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyBinding, MouseButton, ParentElement as _, Render, SharedString, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    context_menu::ContextMenuExt,
    divider::Divider,
    h_flex,
    input::TextInput,
    popover::{Popover, PopoverContent},
    popup_menu::PopupMenuExt,
    v_flex, ActiveTheme as _, ContextModal, IconName, Sizable,
};
use serde::Deserialize;

#[derive(Clone, PartialEq, Deserialize)]
struct Info(usize);

actions!(popover_story, [Copy, Paste, Cut, SearchAll, ToggleCheck]);
impl_internal_actions!(popover_story, [Info]);

pub fn init(cx: &mut App) {
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
    input1: Entity<TextInput>,
}

impl Form {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            input1: cx.new(|cx| TextInput::new(window, cx)),
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
            .child(self.input1.clone())
            .child(
                Button::new("submit")
                    .label("Submit")
                    .primary()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(DismissEvent))),
            )
    }
}

pub struct PopupStory {
    focus_handle: FocusHandle,
    form: Entity<Form>,
    checked: bool,
    message: String,
}

impl super::Story for PopupStory {
    fn title() -> &'static str {
        "Popup"
    }

    fn description() -> &'static str {
        "A popup displays content on top of the main page."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl PopupStory {
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

impl Focusable for PopupStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopupStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = self.form.clone();
        let checked = self.checked;

        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_search_all))
            .on_action(cx.listener(Self::on_action_info))
            .on_action(cx.listener(Self::on_action_toggle_check))
            .p_4()
            .mb_5()
            .size_full()
            .min_h(px(400.))
            .context_menu({
                move |this, window, cx| {
                    this.separator()
                        .menu("Cut", Box::new(Cut))
                        .menu("Copy", Box::new(Copy))
                        .menu("Paste", Box::new(Paste))
                        .separator()
                        .menu_with_check("Toggle Check", checked, Box::new(ToggleCheck))
                        .separator()
                        .submenu("Settings", window, cx, move |menu, _, _| {
                            menu.menu("Info 0", Box::new(Info(0)))
                                .separator()
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
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_4().child(
                            Popover::new("info-top-left")
                                .trigger(Button::new("info-top-left").label("Top Left"))
                                .content(|window, cx| {
                                    cx.new(|cx| {
                                        PopoverContent::new(window, cx, |_, _| {
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
                            // .anchor(Corner::TopRight)
                            .trigger(Button::new("info-top-right").label("Top Right"))
                            .content(|window, cx| {
                                cx.new(|cx| {
                                    PopoverContent::new(window, cx, |_, _| {
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
                            .popup_menu(move |this, window, cx| {
                                this.menu("Copy", Box::new(Copy))
                                    .menu("Cut", Box::new(Cut))
                                    .menu("Paste", Box::new(Paste))
                                    .separator()
                                    .menu_with_check("Toggle Check", checked, Box::new(ToggleCheck))
                                    .separator()
                                    .menu_with_icon("Search", IconName::Search, Box::new(SearchAll))
                                    .separator()
                                    .menu_element(Box::new(Info(0)), |_, cx| {
                                        v_flex().child("Custom Element").child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("THis is sub-title"),
                                        )
                                    })
                                    .menu_element_with_check(checked, Box::new(Info(0)), |_, cx| {
                                        h_flex().gap_1().child("Custom Element").child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("checked"),
                                        )
                                    })
                                    .menu_element_with_icon(
                                        IconName::Info,
                                        Box::new(Info(0)),
                                        |_, cx| {
                                            h_flex().gap_1().child("Custom").child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("element"),
                                            )
                                        },
                                    )
                                    .separator()
                                    .submenu("Links", window, cx, |menu, _, _| {
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
                            .popup_menu_with_anchor(Corner::TopRight, move |this, _, _| {
                                let mut this = this.scrollable().max_h(px(300.));
                                for i in 0..100 {
                                    this = this.menu(
                                        SharedString::from(format!("Item {}", i)),
                                        Box::new(Info(i)),
                                    )
                                }
                                this.min_w(px(100.))
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
                                // .anchor(Corner::BottomLeft)
                                .trigger(Button::new("pop").label("Popup with Form").w(px(300.)))
                                .content(move |_, _| form.clone()),
                        )
                        .child(
                            Popover::new("info-bottom-right")
                                // .anchor(Corner::BottomRight)
                                .mouse_button(MouseButton::Right)
                                .trigger(Button::new("pop").label("Mouse Right Click").w(px(300.)))
                                .content(|window, cx| {
                                    cx.new(|cx| {
                                        PopoverContent::new(window, cx, |_, cx| {
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
                                                                    |_, _, window, cx| {
                                                                        window.push_notification(
                                                                            "You have clicked Ok.",
                                                                            cx,
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
                                                                    |_, _, _, cx| {
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
