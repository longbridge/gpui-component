use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Global, IntoElement, Render, Window,
};

use gpui_component::setting::{
    SettingField, SettingFieldType, SettingGroup, SettingItem, SettingPage, Settings,
};

struct AppSettings {
    dark_mode: bool,
    auto_switch_theme: bool,
    font_family: String,
    font_size: f64,
    notifications_enabled: bool,
    auto_update: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: false,
            auto_switch_theme: false,
            font_family: "Arial".into(),
            font_size: 14.0,
            notifications_enabled: true,
            auto_update: true,
        }
    }
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
        }
    }

    fn setting_pages(&self, _: &mut Context<Self>) -> Vec<SettingPage> {
        vec![SettingPage::new("Appearance").groups(vec![
            SettingGroup::new("Theme").items(vec![
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
                            AppSettings::global_mut(cx).dark_mode =
                                AppSettings::default().dark_mode;
                        },
                    }),
                },
                SettingItem::Item {
                    id: "auto-switch-theme",
                    label: "Auto Switch Theme".into(),
                    description: Some(
                        "Automatically switch theme based on system appearance.".into(),
                    ),
                    field_type: SettingFieldType::Checkbox,
                    field: Rc::new(SettingField {
                        value: |cx: &App| AppSettings::global(cx).auto_switch_theme,
                        set_value: |val: bool, cx: &mut App| {
                            AppSettings::global_mut(cx).auto_switch_theme = val;
                        },
                        reset_value: |cx: &mut App| {
                            AppSettings::global_mut(cx).auto_switch_theme =
                                AppSettings::default().auto_switch_theme;
                        },
                    }),
                },
            ]),
            SettingGroup::new("Font").items(vec![
                SettingItem::Item {
                    id: "font-family",
                    label: "Font Family".into(),
                    description: Some("Select the font family for the application.".into()),
                    field_type: SettingFieldType::Dropdown {
                        options: vec![
                            ("Arial".into(), "Arial".into()),
                            ("Helvetica".into(), "Helvetica".into()),
                            ("Times New Roman".into(), "Times New Roman".into()),
                            ("Courier New".into(), "Courier New".into()),
                        ],
                    },
                    field: Rc::new(SettingField {
                        value: |cx: &App| AppSettings::global(cx).font_family.clone(),
                        set_value: |val: String, cx: &mut App| {
                            AppSettings::global_mut(cx).font_family = val;
                        },
                        reset_value: |cx: &mut App| {
                            AppSettings::global_mut(cx).font_family =
                                AppSettings::default().font_family;
                        },
                    }),
                },
                SettingItem::Item {
                    id: "font-size",
                    label: "Font Size".into(),
                    description: Some("Adjust the font size for better readability.".into()),
                    field_type: SettingFieldType::NumberInput {
                        min: 10.0,
                        max: 100.0,
                        step: 5.0,
                    },
                    field: Rc::new(SettingField {
                        value: |cx: &App| AppSettings::global(cx).font_size,
                        set_value: |val: f64, cx: &mut App| {
                            AppSettings::global_mut(cx).font_size = val;
                        },
                        reset_value: |cx: &mut App| {
                            AppSettings::global_mut(cx).font_size =
                                AppSettings::default().font_size;
                        },
                    }),
                },
            ]),
        ])]
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
