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

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod system_hotkey {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static HOTKEY_MANAGER: OnceLock<Mutex<GlobalHotKeyManager>> = OnceLock::new();
    static TOGGLE_HOTKEY_ID: OnceLock<u32> = OnceLock::new();

    pub fn register() {
        // 初始化 manager（只执行一次）
        let manager = HOTKEY_MANAGER.get_or_init(|| {
            let manager = GlobalHotKeyManager::new()
                .map_err(|err| {
                    tracing::warn!("系统级热键管理器初始化失败: {err:?}");
                })
                .ok()
                .expect("GlobalHotKeyManager 初始化失败");

            Mutex::new(manager)
        });

        // 构建热键
        let hotkey = build_toggle_hotkey();
        let hotkey_id = hotkey.id();

        // 注册热键
        if let Err(err) = manager.lock().unwrap().register(hotkey) {
            tracing::warn!("系统级热键注册失败: {err:?}");
            return;
        }

        // 记录 hotkey id（只会成功一次）
        let _ = TOGGLE_HOTKEY_ID.set(hotkey_id);

        // 设置事件监听（覆盖式即可，无需 Once）
        GlobalHotKeyEvent::set_event_handler(Some(handle_hotkey_event));
    }

    fn handle_hotkey_event(event: GlobalHotKeyEvent) {
        let Some(&registered_id) = TOGGLE_HOTKEY_ID.get() else {
            return;
        };

        if should_dispatch_hotkey_event(event.id, registered_id, event.state) {
            if let Err(err) = dispatch_main_window_shortcut() {
                tracing::warn!("主窗口系统级快捷键处理失败: {err:?}");
            }
        }
    }

    pub(crate) fn build_toggle_hotkey() -> HotKey {
        #[cfg(target_os = "macos")]
        {
            HotKey::new(
                Some(HotkeyModifiers::SUPER | HotkeyModifiers::ALT),
                HotkeyCode::KeyM,
            )
        }

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            HotKey::new(Some(HotkeyModifiers::CONTROL), HotkeyCode::Space)
        }
    }

    #[inline]
    pub(crate) fn should_dispatch_hotkey_event(
        received_id: u32,
        registered_id: u32,
        state: HotKeyState,
    ) -> bool {
        received_id == registered_id && state == HotKeyState::Pressed
    }

    #[cfg(target_os = "macos")]
    fn dispatch_main_window_shortcut() -> Result<(), app_visibility::AppVisibilityError> {
        toggle_main_window_visibility()
    }

    #[cfg(target_os = "windows")]
    fn dispatch_main_window_shortcut() -> Result<(), app_visibility::AppVisibilityError> {
        restore_main_window()
    }

    #[cfg(target_os = "linux")]
    fn dispatch_main_window_shortcut() -> Result<(), app_visibility::AppVisibilityError> {
        toggle_main_window_visibility()
    }
}

fn main() {
    if update::handle_update_command() {
        return;
    }

    let app = Application::new()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    app.run(move |cx| {
        #[cfg(target_os = "macos")]
        if let Err(err) = register_activation_observer() {
            tracing::warn!("主窗口激活观察器注册失败: {err:?}");
        }

        onetcli_app::init(cx);

        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
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