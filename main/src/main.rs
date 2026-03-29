#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

rust_i18n::i18n!("locales", fallback = "en");

mod auth;

mod encourage;
mod home;
mod home_tab;
mod license;
mod onetcli_app;
mod setting_tab;
mod settings;
mod update;
mod user_avatar;

use crate::onetcli_app::OnetCliApp;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use app_visibility::register_main_window_handle;
#[cfg(target_os = "macos")]
use app_visibility::{register_activation_observer, toggle_main_window_visibility};
#[cfg(target_os = "windows")]
use app_visibility::restore_main_window;
use db::GlobalDbState;
use db_view::database_view_plugin::DatabaseViewPluginRegistry;
use gpui::*;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code as HotkeyCode, HotKey, Modifiers as HotkeyModifiers},
};
use gpui_component::Root;
use gpui_component_assets::Assets;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use raw_window_handle::HasWindowHandle;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::sync::{Once, OnceLock};

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod system_hotkey {
    use super::*;

    static REGISTER_TOGGLE_HOTKEY: Once = Once::new();
    static HOTKEY_MANAGER: OnceLock<GlobalHotKeyManager> = OnceLock::new();
    static TOGGLE_HOTKEY_ID: OnceLock<u32> = OnceLock::new();

    pub fn register() {
        REGISTER_TOGGLE_HOTKEY.call_once(|| {
            let manager = match GlobalHotKeyManager::new() {
                Ok(manager) => manager,
                Err(err) => {
                    tracing::warn!("系统级热键管理器初始化失败: {err:?}");
                    return;
                }
            };

            let hotkey = build_toggle_hotkey();
            let hotkey_id = hotkey.id();
            if let Err(err) = manager.register(hotkey) {
                tracing::warn!("系统级热键注册失败: {err:?}");
                return;
            }

            if TOGGLE_HOTKEY_ID.set(hotkey_id).is_err() {
                tracing::warn!("系统级热键标识已初始化，跳过重复设置");
            }

            GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
                let Some(registered_hotkey_id) = TOGGLE_HOTKEY_ID.get().copied() else {
                    return;
                };

                if should_dispatch_hotkey_event(
                    event.id,
                    registered_hotkey_id,
                    event.state == HotKeyState::Pressed,
                ) && let Err(err) = dispatch_main_window_shortcut()
                {
                    tracing::warn!("主窗口系统级快捷键处理失败: {err:?}");
                }
            }));

            if HOTKEY_MANAGER.set(manager).is_err() {
                tracing::warn!("系统级热键管理器已初始化，跳过重复注册");
            }
        });
    }

    pub(crate) fn build_toggle_hotkey() -> HotKey {
        #[cfg(target_os = "macos")]
        {
            return HotKey::new(
                Some(HotkeyModifiers::SUPER | HotkeyModifiers::ALT),
                HotkeyCode::KeyM,
            );
        }

        #[cfg(target_os = "windows")]
        {
            return HotKey::new(Some(HotkeyModifiers::CONTROL), HotkeyCode::Space);
        }

        #[allow(unreachable_code)]
        HotKey::new(
            Some(HotkeyModifiers::SUPER | HotkeyModifiers::ALT),
            HotkeyCode::KeyM,
        )
    }

    pub(crate) fn should_dispatch_hotkey_event(
        received_hotkey_id: u32,
        registered_hotkey_id: u32,
        is_pressed: bool,
    ) -> bool {
        received_hotkey_id == registered_hotkey_id && is_pressed
    }

    #[cfg(target_os = "macos")]
    fn dispatch_main_window_shortcut() -> Result<(), app_visibility::AppVisibilityError> {
        toggle_main_window_visibility()
    }

    #[cfg(target_os = "windows")]
    fn dispatch_main_window_shortcut() -> Result<(), app_visibility::AppVisibilityError> {
        restore_main_window()
    }
}

fn main() {
    if update::handle_update_command() {
        return;
    }

    let app = Application::new()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    app.on_reopen(|cx| onetcli_app::reopen_last_window(cx));

    app.run(move |cx| {
        #[cfg(target_os = "macos")]
        if let Err(err) = register_activation_observer() {
            tracing::warn!("主窗口激活观察器注册失败: {err:?}");
        }

        onetcli_app::init(cx);

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        system_hotkey::register();

        setting_tab::init_settings(cx);
        let db_state = GlobalDbState::new();
        db_state.start_cleanup_task(cx);
        cx.set_global(db_state);

        db_view::init_ask_ai_notifier(cx);

        let view_registry = DatabaseViewPluginRegistry::new();
        cx.set_global(view_registry);
        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            #[cfg(not(target_os = "linux"))]
            titlebar: Some(gpui_component::TitleBar::title_bar_options()),
            window_min_size: Some(Size {
                width: px(640.),
                height: px(480.),
            }),
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            kind: WindowKind::Normal,
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                window.activate_window();
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                match window.window_handle() {
                    Ok(window_handle) => {
                        if let Err(err) = register_main_window_handle(window_handle.as_raw()) {
                            tracing::warn!("主窗口句柄注册失败: {err:?}");
                        }
                    }
                    Err(err) => {
                        tracing::warn!("主窗口句柄获取失败: {err:?}");
                    }
                }
                update::schedule_update_check(window, cx);
                let view = cx.new(|cx| OnetCliApp::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::system_hotkey::{build_toggle_hotkey, should_dispatch_hotkey_event};
    use app_visibility::{MainWindowVisibilityAction, build_main_window_visibility_action};

    #[test]
    fn restore_hotkey_uses_cmd_alt_m_on_macos() {
        let hotkey = build_toggle_hotkey();

        assert_eq!(hotkey.to_string(), "alt+super+KeyM");
    }

    #[test]
    fn restore_hotkey_event_only_triggers_for_target_pressed_state() {
        let hotkey = build_toggle_hotkey();
        let hotkey_id = hotkey.id();

        assert!(should_dispatch_hotkey_event(hotkey_id, hotkey_id, true));
        assert!(!should_dispatch_hotkey_event(hotkey_id + 1, hotkey_id, true));
        assert!(!should_dispatch_hotkey_event(hotkey_id, hotkey_id, false));
    }

    #[test]
    fn hotkey_toggle_action_hides_when_app_is_active_and_has_visible_window() {
        assert_eq!(
            build_main_window_visibility_action(true, true),
            MainWindowVisibilityAction::Hide
        );
    }

    #[test]
    fn hotkey_toggle_action_restores_when_app_is_inactive_or_without_visible_window() {
        assert_eq!(
            build_main_window_visibility_action(false, true),
            MainWindowVisibilityAction::Restore
        );
        assert_eq!(
            build_main_window_visibility_action(true, false),
            MainWindowVisibilityAction::Restore
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::system_hotkey::{build_toggle_hotkey, should_dispatch_hotkey_event};

    #[test]
    fn restore_hotkey_uses_ctrl_space_on_windows() {
        let hotkey = build_toggle_hotkey();

        assert_eq!(hotkey.to_string(), "ctrl+Space");
    }

    #[test]
    fn restore_hotkey_event_only_triggers_for_target_pressed_state() {
        let hotkey = build_toggle_hotkey();
        let hotkey_id = hotkey.id();

        assert!(should_dispatch_hotkey_event(hotkey_id, hotkey_id, true));
        assert!(!should_dispatch_hotkey_event(hotkey_id + 1, hotkey_id, true));
        assert!(!should_dispatch_hotkey_event(hotkey_id, hotkey_id, false));
    }
}
