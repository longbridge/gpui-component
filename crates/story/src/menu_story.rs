use gpui::{
    actions, div, impl_internal_actions, px, App, AppContext, Context, Corner, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyBinding, ParentElement as _, Render,
    SharedString, Styled as _, Window,
};
use gpui_component::{
    button::Button, context_menu::ContextMenuExt, h_flex, popup_menu::PopupMenuExt as _, v_flex,
    ActiveTheme as _, IconName,
};
use serde::Deserialize;

use crate::section;

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

pub struct MenuStory {
    focus_handle: FocusHandle,
    checked: bool,
    message: String,
}

impl super::Story for MenuStory {
    fn title() -> &'static str {
        "Menu"
    }

    fn description() -> &'static str {
        "Popup menu and context menu"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl MenuStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.focus_self(window);

        Self {
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

impl Focusable for MenuStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MenuStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.checked;

        v_flex()
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
                section("Popup Menu")
                    .child(Button::new("popup-menu-1").label("Edit").popup_menu(
                        move |this, window, cx| {
                            this.link("About", "https://github.com/longbridge/gpui-component")
                                .separator()
                                .menu("Copy", Box::new(Copy))
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
                                .menu_with_disabled("Disabled Item", Box::new(Info(0)), true)
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
                        },
                    ))
                    .child(self.message.clone()),
            )
            .child(
                section("Context Menu")
                    .child("Right click to open ContextMenu")
                    .min_h_20()
                    .context_menu({
                        move |this, window, cx| {
                            this.external_link_icon(false)
                                .link("About", "https://github.com/longbridge/gpui-component")
                                .separator()
                                .menu("Cut", Box::new(Cut))
                                .menu("Copy", Box::new(Copy))
                                .menu("Paste", Box::new(Paste))
                                .separator()
                                .label("This is a label")
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
                    }),
            )
            .child(
                section("Menu with scrollbar").child(
                    Button::new("popup-menu-11112")
                        .label("Scrollable Menu")
                        .popup_menu_with_anchor(Corner::TopRight, move |this, _, _| {
                            let mut this = this
                                .scrollable()
                                .max_h(px(300.))
                                .label(format!("Total {} items", 100));
                            for i in 0..100 {
                                this = this.menu(
                                    SharedString::from(format!("Item {}", i)),
                                    Box::new(Info(i)),
                                )
                            }
                            this.min_w(px(100.))
                        }),
                ),
            )
    }
}
