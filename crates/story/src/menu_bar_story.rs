use gpui::{
    actions, px, Action, App, AppContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, MenuItem, ParentElement as _, Render, Styled as _,
    Window,
};
use gpui_component::{
    menu::{MenuBar, MenuBarMenu},
    v_flex,
};
use serde::Deserialize;

use crate::section;

#[derive(Action, Clone, PartialEq, Deserialize)]
#[action(namespace = menu_bar_story, no_json)]
struct Info(usize);

actions!(
    menu_bar_story,
    [
        New,
        Open,
        Save,
        Exit,
        Copy,
        Paste,
        Cut,
        SearchAll,
        ToggleCheck
    ]
);

const CONTEXT: &str = "menu_bar_story";
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

pub struct MenuBarStory {
    focus_handle: FocusHandle,
    checked: bool,
    message: String,
}

impl super::Story for MenuBarStory {
    fn title() -> &'static str {
        "MenuBar"
    }

    fn description() -> &'static str {
        "A visually persistent menu common in desktop applications that provides quick access to a consistent set of commands."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl MenuBarStory {
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

impl Focusable for MenuBarStory {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MenuBarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                section("MenuBar").child(MenuBar::new("main-menu").menus(vec![
                    MenuBarMenu::new("File").items(vec![
                        MenuItem::action("New", New).owned(),
                        MenuItem::action("Open", Open).owned(),
                        MenuItem::action("Save", Save).owned(),
                        MenuItem::separator().owned(),
                        MenuItem::action("Exit", Exit).owned(),
                    ]),
                    MenuBarMenu::new("Edit").items(vec![
                        MenuItem::action("Copy", Copy).owned(),
                        MenuItem::action("Cut", Cut).owned(),
                        MenuItem::action("Paste", Paste).owned(),
                        MenuItem::separator().owned(),
                        MenuItem::action("Search All", SearchAll).owned(),
                    ]),
                    MenuBarMenu::new("Window").disabled(true),
                    MenuBarMenu::new("View").items(vec![
                        MenuItem::action("Info 1", Info(1)).owned(),
                        MenuItem::separator().owned(),
                        MenuItem::action("Info 2", Info(2)).owned(),
                        MenuItem::action("Info 3", Info(3)).owned(),
                    ]),
                ])),
            )
            .child(self.message.clone())
    }
}
