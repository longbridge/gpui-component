use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Element, Entity, FocusHandle, Focusable, Global, IntoElement,
    ParentElement as _, Render, SharedString, Styled, Window,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size,
    button::Button,
    group_box::GroupBoxVariant,
    h_flex,
    label::Label,
    setting::{
        NumberFieldOptions, SettingField, SettingFieldType, SettingGroup, SettingItem, SettingPage,
        Settings,
    },
    v_flex,
};

struct AppSettings {
    dark_mode: bool,
    auto_switch_theme: bool,
    cli_path: String,
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
            cli_path: "/usr/local/bin/bash".into(),
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
    group_variant: GroupBoxVariant,
    size: Size,
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
            group_variant: GroupBoxVariant::Outline,
            size: Size::default(),
        }
    }

    fn setting_pages(&self, cx: &mut Context<Self>) -> Vec<SettingPage> {
        let view = cx.entity();
        let default_settings = AppSettings::default();

        vec![
            SettingPage::new("General").groups(vec![
                SettingGroup::new().title("Appearance").items(vec![
                    SettingItem::Item {
                        title: "Dark Mode".into(),
                        description: Some("Switch between light and dark themes.".into()),
                        field_type: SettingFieldType::Switch,
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).dark_mode,
                                |val: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).dark_mode = val;
                                },
                            )
                            .default_value(default_settings.dark_mode),
                        ),
                    },
                    SettingItem::Item {
                        title: "Auto Switch Theme".into(),
                        description: Some(
                            "Automatically switch theme based on system appearance.".into(),
                        ),
                        field_type: SettingFieldType::Checkbox,
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).auto_switch_theme,
                                |val: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).auto_switch_theme = val;
                                },
                            )
                            .default_value(default_settings.auto_switch_theme),
                        ),
                    },
                    SettingItem::Item {
                        title: "Group Variant".into(),
                        description: Some("Select the variant for setting groups.".into()),
                        field_type: SettingFieldType::Dropdown {
                            options: vec![
                                (GroupBoxVariant::Normal.as_str().into(), "Normal".into()),
                                (GroupBoxVariant::Outline.as_str().into(), "Outline".into()),
                                (GroupBoxVariant::Fill.as_str().into(), "Fill".into()),
                            ],
                        },
                        field: Rc::new(
                            SettingField::new(
                                {
                                    let view = view.clone();
                                    move |cx: &App| {
                                        SharedString::from(
                                            view.read(cx).group_variant.as_str().to_string(),
                                        )
                                    }
                                },
                                {
                                    let view = view.clone();
                                    move |val: SharedString, cx: &mut App| {
                                        view.update(cx, |view, cx| {
                                            view.group_variant =
                                                GroupBoxVariant::from_str(val.as_str());
                                            cx.notify();
                                        });
                                    }
                                },
                            )
                            .default_value(GroupBoxVariant::Outline.as_str().to_string().into()),
                        ),
                    },
                    SettingItem::Item {
                        title: "Group Field Size".into(),
                        description: Some("Set the field control size in the settings.".into()),
                        field_type: SettingFieldType::Dropdown {
                            options: vec![
                                (Size::Medium.as_str().into(), "Medium".into()),
                                (Size::Small.as_str().into(), "Small".into()),
                                (Size::XSmall.as_str().into(), "XSmall".into()),
                            ],
                        },
                        field: Rc::new(
                            SettingField::new(
                                {
                                    let view = view.clone();
                                    move |cx: &App| {
                                        SharedString::from(view.read(cx).size.as_str().to_string())
                                    }
                                },
                                {
                                    let view = view.clone();
                                    move |val: SharedString, cx: &mut App| {
                                        view.update(cx, |view, cx| {
                                            view.size = Size::from_str(val.as_str());
                                            cx.notify();
                                        });
                                    }
                                },
                            )
                            .default_value(Size::default().as_str().to_string().into()),
                        ),
                    },
                ]),
                SettingGroup::new().title("Font").items(vec![
                    SettingItem::Item {
                        title: "Font Family".into(),
                        description: Some("Select the font family for the application.".into()),
                        field_type: SettingFieldType::Dropdown {
                            options: vec![
                                ("Arial".into(), "Arial".into()),
                                ("Helvetica".into(), "Helvetica".into()),
                                ("Times New Roman".into(), "Times New Roman".into()),
                                ("Courier New".into(), "Courier New".into()),
                            ],
                        },
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).font_family.clone(),
                                |val: String, cx: &mut App| {
                                    AppSettings::global_mut(cx).font_family = val;
                                },
                            )
                            .default_value(default_settings.font_family),
                        ),
                    },
                    SettingItem::Item {
                        title: "Font Size".into(),
                        description: Some("Adjust the font size for better readability.".into()),
                        field_type: SettingFieldType::NumberInput {
                            options: NumberFieldOptions {
                                min: 8.0,
                                max: 72.0,
                                ..Default::default()
                            },
                        },
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).font_size,
                                |val: f64, cx: &mut App| {
                                    AppSettings::global_mut(cx).font_size = val;
                                },
                            )
                            .default_value(default_settings.font_size),
                        ),
                    },
                ]),
                SettingGroup::new().title("Other").items(vec![
                    SettingItem::Element {
                        render: Rc::new(|_, _| {
                            h_flex()
                                .w_full()
                                .justify_between()
                                .gap_3()
                                .child("This is a custom element line.")
                                .child(
                                    Button::new("action")
                                        .icon(IconName::Globe)
                                        .label("Repository...")
                                        .outline()
                                        .on_click(|_, _, cx| {
                                            cx.open_url(
                                                "https://github.com/longbridge/gpui-component",
                                            );
                                        }),
                                )
                                .into_any_element()
                        }),
                    },
                    SettingItem::Item {
                        title: "CLI Path".into(),
                        description: Some(
                            "Set the path to the command-line interface executable.".into(),
                        ),
                        field_type: SettingFieldType::Input,
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).cli_path.clone(),
                                |val: String, cx: &mut App| {
                                    println!("cli-path set value: {}", val);
                                    AppSettings::global_mut(cx).cli_path = val;
                                },
                            )
                            .default_value(default_settings.cli_path),
                        ),
                    },
                ]),
            ]),
            SettingPage::new("Software Update").groups(vec![
                SettingGroup::new().title("Updates").items(vec![
                    SettingItem::Item {
                        title: "Enable Notifications".into(),
                        description: Some("Receive notifications about updates and news.".into()),
                        field_type: SettingFieldType::Switch,
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).notifications_enabled,
                                |val: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).notifications_enabled = val;
                                },
                            )
                            .default_value(default_settings.notifications_enabled),
                        ),
                    },
                    SettingItem::Item {
                        title: "Auto Update".into(),
                        description: Some("Automatically download and install updates.".into()),
                        field_type: SettingFieldType::Switch,
                        field: Rc::new(
                            SettingField::new(
                                |cx: &App| AppSettings::global(cx).auto_update,
                                |val: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).auto_update = val;
                                },
                            )
                            .default_value(default_settings.auto_update),
                        ),
                    },
                ]),
            ]),
            SettingPage::new("About").groups(vec![SettingGroup::new().items(vec![
                SettingItem::Element {
                    render: Rc::new(|_, cx| {
                        v_flex()
                            .gap_3()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::GalleryVerticalEnd).size_16())
                            .child("GPUI Component")
                            .child(
                                Label::new(
                                    "Rust GUI components for building fantastic cross-platform \
                                    desktop application by using GPUI.",
                                )
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                            )
                            .into_any()
                    }),
                },
            ])]),
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
        Settings::new("app-settings")
            .with_size(self.size)
            .with_group_variant(self.group_variant)
            .pages(self.setting_pages(cx))
    }
}
