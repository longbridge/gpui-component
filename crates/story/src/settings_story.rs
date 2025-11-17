use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Global, IntoElement, ParentElement,
    Render, Window,
};

use gpui_component::{
    setting::{SettingField, SettingFieldType, SettingGroup, SettingItem, SettingPage, Settings},
    switch::Switch,
};

#[derive(Default)]
struct AppSettings {
    dark_mode: bool,
    notifications_enabled: bool,
    auto_update: bool,
}

impl Global for AppSettings {}

impl AppSettings {
    fn global(cx: &App) -> &AppSettings {
        cx.global::<AppSettings>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppSettings {
        cx.global_mut::<AppSettings>()
    }
}

pub struct SettingsStory {
    focus_handle: FocusHandle,
    switch1: bool,
    switch2: bool,
    switch3: bool,
}

impl super::Story for SettingsStory {
    fn title() -> &'static str {
        "Settings"
    }

    fn description() -> &'static str {
        "A collection of settings groups and items for the application."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SettingsStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.set_global::<AppSettings>(AppSettings::default());

        Self {
            focus_handle: cx.focus_handle(),
            switch1: true,
            switch2: false,
            switch3: true,
        }
    }

    fn setting_pages(&self, cx: &mut Context<Self>) -> Vec<SettingPage> {
        vec![
            SettingPage::new("Appearance").group(SettingGroup::new("Theme").items(vec![
                SettingItem::Item {
                    id: "dark-mode",
                    label: "Dark Mode".into(),
                    description: Some("Switch between light and dark themes.".into()),
                    field_type: SettingFieldType::Switch,
                    field: Rc::new(SettingField {
                        value: |cx: &App| AppSettings::global(cx).dark_mode,
                        set_value: |val: bool, cx: &mut App| {
                            AppSettings::global_mut(cx).dark_mode = val;
                        },
                        reset_value: |cx: &mut App| {
                            AppSettings::global_mut(cx).dark_mode = false;
                        },
                    }),
                },
            ])),
        ]
    }
}

impl Focusable for SettingsStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Settings::new("app-settings", self.setting_pages(cx))
    }
}
