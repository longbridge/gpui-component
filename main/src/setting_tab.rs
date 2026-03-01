use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use gpui::{
    App, AppContext, AsyncApp, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Render, SharedString,
    Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, Theme, ThemeMode, WindowExt,
    button::{Button, ButtonVariants as _},
    group_box::GroupBoxVariant,
    h_flex,
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};
use one_core::cloud_sync::GlobalCloudUser;
use one_core::cloud_sync::UserInfo;
use one_core::storage::manager::get_config_dir;
use one_core::tab_container::{TabContent, TabContentEvent};
use one_core::utils::auto_save_config::AutoSaveConfig;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::auth::get_auth_service;
use crate::license::{get_license_service, offline_license_public_key};
use crate::settings::llm_providers_view::LlmProvidersView;

// ============================================================================
// 全局用户状态
// ============================================================================

/// 全局当前用户状态
///
/// 用于在设置面板中显示用户信息和执行登出操作。
#[derive(Clone, Default)]
pub struct GlobalCurrentUser {
    user: Arc<RwLock<Option<UserInfo>>>,
}

impl gpui::Global for GlobalCurrentUser {}

impl GlobalCurrentUser {
    /// 获取当前用户
    pub fn get_user(cx: &App) -> Option<UserInfo> {
        if let Some(state) = cx.try_global::<GlobalCurrentUser>() {
            state.user.read().ok().and_then(|u| u.clone())
        } else {
            None
        }
    }

    /// 设置当前用户
    pub fn set_user(user: Option<UserInfo>, cx: &mut App) {
        if !cx.has_global::<GlobalCurrentUser>() {
            cx.set_global(GlobalCurrentUser::default());
        }
        if let Some(state) = cx.try_global::<GlobalCurrentUser>() {
            if let Ok(mut guard) = state.user.write() {
                *guard = user.clone();
            }
        }
        GlobalCloudUser::set_user(user, cx);
    }
}

// ============================================================================
// 数据库配置
// ============================================================================

/// 数据库打开方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DatabaseOpenMode {
    /// 单库模式：每个数据库单独打开一个标签页
    #[default]
    Single,
    /// 工作区模式：按工作区分组打开，同一工作区的数据库在同一标签页
    Workspace,
}

impl DatabaseOpenMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseOpenMode::Single => "single",
            DatabaseOpenMode::Workspace => "workspace",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "workspace" => DatabaseOpenMode::Workspace,
            _ => DatabaseOpenMode::Single,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub theme_mode: String,
    #[serde(default)]
    pub auto_switch_theme: bool,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub database_open_mode: DatabaseOpenMode,
    /// 是否启用SQL查询的自动保存功能
    #[serde(default = "default_true")]
    pub enable_sql_auto_save: bool,
    /// SQL查询自动保存的间隔（秒），默认5秒
    #[serde(default = "default_auto_save_interval")]
    pub sql_auto_save_interval: f64,
}

fn default_font_family() -> String {
    "Arial".to_string()
}

fn default_font_size() -> f64 {
    14.0
}

fn default_true() -> bool {
    true
}

fn default_auto_save_interval() -> f64 {
    5.0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: "zh-CN".to_string(),
            theme_mode: "light".to_string(),
            auto_switch_theme: false,
            font_family: default_font_family(),
            font_size: default_font_size(),
            auto_update: true,
            database_open_mode: DatabaseOpenMode::default(),
            enable_sql_auto_save: true,
            sql_auto_save_interval: default_auto_save_interval(),
        }
    }
}

impl gpui::Global for AppSettings {}

impl AppSettings {
    pub fn global(cx: &App) -> &AppSettings {
        cx.global::<AppSettings>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppSettings {
        cx.global_mut::<AppSettings>()
    }

    fn config_path() -> Option<PathBuf> {
        get_config_dir().ok().map(|dir| dir.join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => {
                    info!("Settings loaded from {:?}", path);
                    settings
                }
                Err(e) => {
                    error!("Failed to parse settings: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                error!("Failed to read settings file: {}", e);
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            error!("Could not determine config path");
            return;
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Failed to create config directory: {}", e);
                return;
            }
        }

        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    error!("Failed to write settings file: {}", e);
                } else {
                    info!("Settings saved to {:?}", path);
                }
            }
            Err(e) => {
                error!("Failed to serialize settings: {}", e);
            }
        }
    }

    pub fn apply(&self, cx: &mut App) {
        gpui_component::set_locale(&self.locale);

        let mode = if self.theme_mode == "dark" {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        Theme::global_mut(cx).mode = mode;
        Theme::change(mode, None, cx);

        // 同步自动保存配置
        self.sync_auto_save_config(cx);
    }

    /// 同步自动保存配置到全局状态
    pub fn sync_auto_save_config(&self, cx: &mut App) {
        Self::update_auto_save_config(self.enable_sql_auto_save, self.sql_auto_save_interval, cx);
    }

    /// 更新自动保存配置（静态方法，避免借用冲突）
    pub fn update_auto_save_config(enabled: bool, interval_seconds: f64, cx: &mut App) {
        if let Some(config) = cx.try_global::<AutoSaveConfig>() {
            config.set_enabled(enabled);
            config.set_interval_seconds(interval_seconds);
        }
    }
}

pub fn init_settings(cx: &mut App) {
    let settings = AppSettings::load();
    // 初始化自动保存配置全局状态
    cx.set_global(AutoSaveConfig::new(
        settings.enable_sql_auto_save,
        settings.sql_auto_save_interval,
    ));
    settings.apply(cx);
    cx.set_global(settings);
}

pub struct SettingsPanel {
    focus_handle: FocusHandle,
    llm_providers_view: Entity<LlmProvidersView>,
    size: Size,
    group_variant: GroupBoxVariant,
}

impl SettingsPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let llm_providers_view = cx.new(|cx| LlmProvidersView::new(cx));
        Self {
            focus_handle: cx.focus_handle(),
            llm_providers_view,
            size: Size::default(),
            group_variant: GroupBoxVariant::Outline,
        }
    }

    fn setting_pages(&self, _window: &mut Window, _cx: &App) -> Vec<SettingPage> {
        let llm_view = self.llm_providers_view.clone();
        let default_settings = AppSettings::default();

        vec![
            SettingPage::new(t!("Settings.General.title"))
                .resettable(true)
                .default_open(true)
                .groups(vec![
                    SettingGroup::new()
                        .title(t!("Settings.General.Language.group_title"))
                        .items(vec![
                            SettingItem::new(
                                t!("Settings.General.Language.ui_language"),
                                SettingField::dropdown(
                                    vec![
                                        (
                                            "zh-CN".into(),
                                            t!("Settings.General.Language.zh_cn").into(),
                                        ),
                                        (
                                            "zh-HK".into(),
                                            t!("Settings.General.Language.zh_hk").into(),
                                        ),
                                        ("en".into(), t!("Settings.General.Language.en").into()),
                                    ],
                                    |cx: &App| {
                                        SharedString::from(AppSettings::global(cx).locale.clone())
                                    },
                                    |val: SharedString, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.locale = val.to_string();
                                        gpui_component::set_locale(&settings.locale);
                                        settings.save();
                                    },
                                )
                                .default_value(SharedString::from(default_settings.locale)),
                            )
                            .description(
                                t!("Settings.General.Language.ui_language_desc").to_string(),
                            ),
                        ]),
                    SettingGroup::new()
                        .title(t!("Settings.General.Appearance.group_title"))
                        .items(vec![
                            SettingItem::new(
                                t!("Settings.General.Appearance.dark_mode"),
                                SettingField::switch(
                                    |cx: &App| cx.theme().mode.is_dark(),
                                    |val: bool, cx: &mut App| {
                                        let mode = if val {
                                            ThemeMode::Dark
                                        } else {
                                            ThemeMode::Light
                                        };
                                        Theme::global_mut(cx).mode = mode;
                                        Theme::change(mode, None, cx);

                                        let settings = AppSettings::global_mut(cx);
                                        settings.theme_mode = if val {
                                            "dark".to_string()
                                        } else {
                                            "light".to_string()
                                        };
                                        settings.save();
                                    },
                                )
                                .default_value(false),
                            )
                            .description(
                                t!("Settings.General.Appearance.dark_mode_desc").to_string(),
                            ),
                            SettingItem::new(
                                t!("Settings.General.Appearance.auto_switch_theme"),
                                SettingField::checkbox(
                                    |cx: &App| AppSettings::global(cx).auto_switch_theme,
                                    |val: bool, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.auto_switch_theme = val;
                                        settings.save();
                                    },
                                )
                                .default_value(default_settings.auto_switch_theme),
                            )
                            .description(
                                t!("Settings.General.Appearance.auto_switch_theme_desc")
                                    .to_string(),
                            ),
                        ]),
                    SettingGroup::new()
                        .title(t!("Settings.General.Font.group_title"))
                        .item(
                            SettingItem::new(
                                t!("Settings.General.Font.font_family"),
                                SettingField::dropdown(
                                    vec![
                                        ("Arial".into(), "Arial".into()),
                                        ("Helvetica".into(), "Helvetica".into()),
                                        ("Times New Roman".into(), "Times New Roman".into()),
                                        ("Courier New".into(), "Courier New".into()),
                                    ],
                                    |cx: &App| {
                                        SharedString::from(
                                            AppSettings::global(cx).font_family.clone(),
                                        )
                                    },
                                    |val: SharedString, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.font_family = val.to_string();
                                        settings.save();
                                    },
                                )
                                .default_value(SharedString::from(default_settings.font_family)),
                            )
                            .description(t!("Settings.General.Font.font_family_desc").to_string()),
                        )
                        .item(
                            SettingItem::new(
                                t!("Settings.General.Font.font_size"),
                                SettingField::number_input(
                                    NumberFieldOptions {
                                        min: 8.0,
                                        max: 72.0,
                                        ..Default::default()
                                    },
                                    |cx: &App| AppSettings::global(cx).font_size,
                                    |val: f64, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.font_size = val;
                                        settings.save();
                                    },
                                )
                                .default_value(default_settings.font_size),
                            )
                            .description(t!("Settings.General.Font.font_size_desc").to_string()),
                        ),
                    SettingGroup::new()
                        .title(t!("Settings.General.Database.group_title"))
                        .items(vec![
                            SettingItem::new(
                                t!("Settings.General.Database.open_mode"),
                                SettingField::dropdown(
                                    vec![
                                        (
                                            "single".into(),
                                            t!("Settings.General.Database.open_mode_single").into(),
                                        ),
                                        (
                                            "workspace".into(),
                                            t!("Settings.General.Database.open_mode_workspace")
                                                .into(),
                                        ),
                                    ],
                                    |cx: &App| {
                                        SharedString::from(
                                            AppSettings::global(cx).database_open_mode.as_str(),
                                        )
                                    },
                                    |val: SharedString, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.database_open_mode =
                                            DatabaseOpenMode::from_str(&val);
                                        settings.save();
                                    },
                                )
                                .default_value(SharedString::from(
                                    default_settings.database_open_mode.as_str(),
                                )),
                            )
                            .description(
                                t!("Settings.General.Database.open_mode_desc").to_string(),
                            ),
                            SettingItem::new(
                                t!("Settings.General.Database.auto_save"),
                                SettingField::switch(
                                    |cx: &App| AppSettings::global(cx).enable_sql_auto_save,
                                    |val: bool, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.enable_sql_auto_save = val;
                                        settings.save();
                                        AppSettings::update_auto_save_config(
                                            val,
                                            cx.global::<AppSettings>().sql_auto_save_interval,
                                            cx,
                                        );
                                    },
                                )
                                .default_value(default_settings.enable_sql_auto_save),
                            )
                            .description(
                                t!("Settings.General.Database.auto_save_desc").to_string(),
                            ),
                            SettingItem::new(
                                t!("Settings.General.Database.auto_save_interval"),
                                SettingField::number_input(
                                    NumberFieldOptions {
                                        min: 1.0,
                                        max: 60.0,
                                        step: 1.0,
                                    },
                                    |cx: &App| AppSettings::global(cx).sql_auto_save_interval,
                                    |val: f64, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.sql_auto_save_interval = val;
                                        settings.save();
                                        AppSettings::update_auto_save_config(
                                            cx.global::<AppSettings>().enable_sql_auto_save,
                                            val,
                                            cx,
                                        );
                                    },
                                )
                                .default_value(default_settings.sql_auto_save_interval),
                            )
                            .description(
                                t!("Settings.General.Database.auto_save_interval_desc").to_string(),
                            ),
                        ]),
                ]),
            SettingPage::new(t!("LlmProviders.title")).group(SettingGroup::new().item(
                SettingItem::render(move |_options, _window, _cx| {
                    llm_view.clone().into_any_element()
                }),
            )),
            // 账户设置页
            SettingPage::new(t!("Settings.Account.title")).group(SettingGroup::new().item(
                SettingItem::render(move |_options, window, cx| render_account_section(window, cx)),
            )),
        ]
    }
}

impl Focusable for SettingsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabContentEvent> for SettingsPanel {}

impl TabContent for SettingsPanel {
    fn content_key(&self) -> &'static str {
        "Settings"
    }

    fn title(&self, _cx: &App) -> SharedString {
        SharedString::from(t!("Common.settings"))
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::Settings.mono())
    }

    fn closeable(&self, _cx: &App) -> bool {
        true
    }

    fn on_activate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if !cx.has_global::<AppSettings>() {
            init_settings(cx);
        }
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !cx.has_global::<AppSettings>() {
            init_settings(cx);
        }

        div().track_focus(&self.focus_handle).size_full().child(
            Settings::new("main-app-settings")
                .with_size(self.size)
                .with_group_variant(self.group_variant)
                .pages(self.setting_pages(window, cx)),
        )
    }
}

/// 渲染账户设置区域
fn render_account_section(_window: &mut Window, cx: &App) -> gpui::AnyElement {
    let user = GlobalCurrentUser::get_user(cx);

    if let Some(user) = user {
        // 已登录状态：显示用户信息和登出按钮
        let email: SharedString = user.email.clone().into();
        let display_name: SharedString = user
            .username
            .clone()
            .unwrap_or_else(|| {
                user.email
                    .split('@')
                    .next()
                    .unwrap_or(&user.email)
                    .to_string()
            })
            .into();

        v_flex()
            .gap_4()
            .p_4()
            // 用户信息区域
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("Settings.Account.username").to_string()),
                            )
                            .child(div().text_sm().child(display_name)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(t!("Settings.Account.email").to_string()),
                            )
                            .child(div().text_sm().child(email)),
                    ),
            )
            // 登出按钮
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("import-license-button")
                            .icon(IconName::File)
                            .label("导入离线 License")
                            .on_click(move |_, window, cx| {
                                let public_key = match offline_license_public_key() {
                                    Ok(key) => key,
                                    Err(msg) => {
                                        window.push_notification(msg, cx);
                                        return;
                                    }
                                };
                                let license_service = get_license_service(cx);
                                let future = cx.prompt_for_paths(PathPromptOptions {
                                    files: true,
                                    directories: false,
                                    multiple: false,
                                    prompt: Some("选择 License 文件".into()),
                                });

                                window
                                    .spawn(cx, async move |cx| {
                                        if let Ok(Ok(Some(paths))) = future.await {
                                            if let Some(path) = paths.into_iter().next() {
                                                let result = license_service
                                                    .import_offline_license_from_path(
                                                        &path,
                                                        &public_key,
                                                        None,
                                                    );
                                                let message = match result {
                                                    Ok(_) => "离线 License 导入成功".to_string(),
                                                    Err(err) => {
                                                        format!("离线 License 导入失败: {}", err)
                                                    }
                                                };
                                                let _ = cx.update(|_view, cx: &mut App| {
                                                    if let Some(window_id) = cx.active_window() {
                                                        let _ = cx.update_window(
                                                            window_id,
                                                            |_, window, cx| {
                                                                window
                                                                    .push_notification(message, cx);
                                                            },
                                                        );
                                                    }
                                                });
                                            }
                                        }
                                    })
                                    .detach();
                            }),
                    )
                    .child(
                        Button::new("logout-button")
                            .icon(IconName::Close)
                            .label(t!("Auth.logout"))
                            .danger()
                            .on_click(move |_, _window, cx| {
                                // 清除 License
                                get_license_service(cx).clear();

                                // 执行登出
                                let auth = get_auth_service(cx);
                                cx.spawn(async move |cx: &mut AsyncApp| {
                                    auth.sign_out().await;
                                    cx.update(|cx| {
                                        GlobalCurrentUser::set_user(None, cx);
                                    });
                                })
                                .detach();
                            }),
                    ),
            )
            .into_any_element()
    } else {
        // 未登录状态：显示提示信息
        v_flex()
            .gap_2()
            .p_4()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Settings.Account.not_logged_in").to_string()),
            )
            .into_any_element()
    }
}
