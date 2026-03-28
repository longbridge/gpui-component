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
use db::GlobalDbState;
use db_view::database_view_plugin::DatabaseViewPluginRegistry;
use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;

#[cfg(target_os = "macos")]
mod macos_activation_restore {
    use cocoa::base::{id, nil, BOOL, YES};
    use cocoa::foundation::{NSString, NSUInteger};
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::OnceLock;

    static OBSERVER_CLASS: OnceLock<&'static Class> = OnceLock::new();
    static mut OBSERVER_INSTANCE: id = nil;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum HotkeyToggleAction {
        Hide,
        Restore,
    }

    pub fn register() {
        unsafe {
            let observer_class = *OBSERVER_CLASS.get_or_init(|| {
                let superclass = class!(NSObject);
                let mut decl =
                    ClassDecl::new("OnetCliActivationObserver", superclass).expect("观察器类初始化失败");
                decl.add_method(
                    sel!(applicationDidBecomeActive:),
                    application_did_become_active as extern "C" fn(&Object, Sel, id),
                );
                decl.register()
            });

            if OBSERVER_INSTANCE != nil {
                return;
            }

            let observer: id = msg_send![observer_class, new];
            let notification_center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let name = NSString::alloc(nil).init_str("NSApplicationDidBecomeActiveNotification");
            let _: () = msg_send![notification_center, addObserver: observer
                selector: sel!(applicationDidBecomeActive:)
                name: name
                object: app
            ];
            OBSERVER_INSTANCE = observer;
        }
    }

    extern "C" fn application_did_become_active(_: &Object, _: Sel, _: id) {
        restore_or_activate_app();
    }

    pub(super) fn restore_or_activate_app() {
        unsafe {
            if !restore_first_minimized_window() {
                activate_app();
            }
        }
    }

    pub(super) fn toggle_app_visibility() {
        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let app_is_active = app_is_active(app);
            let has_visible_window = has_visible_window(app);

            match build_hotkey_toggle_action(app_is_active, has_visible_window) {
                HotkeyToggleAction::Hide => hide_app(app),
                HotkeyToggleAction::Restore => {
                    if !restore_first_minimized_window() {
                        activate_app();
                    }
                }
            }
        }
    }

    pub(super) fn build_hotkey_toggle_action(
        app_is_active: bool,
        has_visible_window: bool,
    ) -> HotkeyToggleAction {
        if app_is_active && has_visible_window {
            HotkeyToggleAction::Hide
        } else {
            HotkeyToggleAction::Restore
        }
    }

    unsafe fn restore_first_minimized_window() -> bool {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let windows: id = msg_send![app, windows];
        let count: NSUInteger = msg_send![windows, count];

        for index in 0..count {
            let window: id = msg_send![windows, objectAtIndex: index];
            let is_miniaturized: BOOL = msg_send![window, isMiniaturized];
            if is_miniaturized == YES {
                let _: () = msg_send![app, unhide: nil];
                let _: () = msg_send![window, deminiaturize: nil];
                let _: () = msg_send![window, makeKeyAndOrderFront: nil];
                let _: () = msg_send![app, activateIgnoringOtherApps: YES];
                return true;
            }
        }

        false
    }

    unsafe fn app_is_active(app: id) -> bool {
        let is_active: BOOL = msg_send![app, isActive];
        is_active == YES
    }

    unsafe fn has_visible_window(app: id) -> bool {
        let windows: id = msg_send![app, windows];
        let count: NSUInteger = msg_send![windows, count];

        for index in 0..count {
            let window: id = msg_send![windows, objectAtIndex: index];
            let is_miniaturized: BOOL = msg_send![window, isMiniaturized];
            if is_miniaturized != YES {
                return true;
            }
        }

        false
    }

    unsafe fn activate_app() {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, unhide: nil];
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    }

    unsafe fn hide_app(app: id) {
        let _: () = msg_send![app, hide: nil];
    }
}

#[cfg(target_os = "macos")]
mod macos_global_hotkey {
    use global_hotkey::{
        GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
        hotkey::{Code, HotKey, Modifiers},
    };
    use std::sync::{Once, OnceLock};

    static REGISTER_RESTORE_HOTKEY: Once = Once::new();
    static HOTKEY_MANAGER: OnceLock<GlobalHotKeyManager> = OnceLock::new();

    pub fn register() {
        REGISTER_RESTORE_HOTKEY.call_once(|| {
            let hotkey = build_restore_hotkey();
            let hotkey_id = restore_hotkey_id(&hotkey);

            GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
                if should_restore_for_hotkey_event(
                    event.id(),
                    hotkey_id,
                    matches!(event.state(), HotKeyState::Pressed),
                ) {
                    super::macos_activation_restore::toggle_app_visibility();
                }
            }));

            let manager = match GlobalHotKeyManager::new() {
                Ok(manager) => manager,
                Err(err) => {
                    tracing::warn!("macOS 系统级恢复热键管理器初始化失败: {err:?}");
                    return;
                }
            };

            if let Err(err) = manager.register(hotkey) {
                tracing::warn!("macOS 系统级恢复热键注册失败: {err:?}");
                return;
            }

            if HOTKEY_MANAGER.set(manager).is_err() {
                tracing::warn!("macOS 系统级恢复热键管理器已初始化，跳过重复注册");
            }
        });
    }

    pub(crate) fn build_restore_hotkey() -> HotKey {
        HotKey::new(Some(Modifiers::SUPER | Modifiers::ALT), Code::KeyM)
    }

    pub(crate) fn restore_hotkey_id(hotkey: &HotKey) -> u32 {
        hotkey.id()
    }

    pub(crate) fn should_restore_for_hotkey_event(
        event_hotkey_id: u32,
        target_hotkey_id: u32,
        is_pressed: bool,
    ) -> bool {
        is_pressed && event_hotkey_id == target_hotkey_id
    }
}

fn main() {
    if update::handle_update_command() {
        return;
    }

    let app = Application::new()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    #[cfg(target_os = "macos")]
    app.on_reopen(|cx| onetcli_app::reopen_last_window(cx));

    app.run(move |cx| {
        #[cfg(target_os = "macos")]
        macos_activation_restore::register();
        onetcli_app::init(cx);
        #[cfg(target_os = "macos")]
        macos_global_hotkey::register();
        setting_tab::init_settings(cx);
        // Initialize global database state
        let db_state = GlobalDbState::new();
        // Start cleanup task
        db_state.start_cleanup_task(cx);
        cx.set_global(db_state);

        // Initialize Ask AI notifier
        db_view::init_ask_ai_notifier(cx);

        // Initialize database view plugin registry
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
    use super::macos_activation_restore::{HotkeyToggleAction, build_hotkey_toggle_action};
    use super::macos_global_hotkey::{
        build_restore_hotkey, restore_hotkey_id, should_restore_for_hotkey_event,
    };

    #[test]
    fn restore_hotkey_uses_cmd_alt_m_on_macos() {
        let hotkey = build_restore_hotkey();

        assert_eq!(hotkey.to_string(), "alt+super+KeyM");
    }

    #[test]
    fn restore_hotkey_event_only_triggers_for_target_pressed_state() {
        let hotkey = build_restore_hotkey();
        let hotkey_id = restore_hotkey_id(&hotkey);

        assert!(should_restore_for_hotkey_event(hotkey_id, hotkey_id, true));
        assert!(!should_restore_for_hotkey_event(hotkey_id + 1, hotkey_id, true));
        assert!(!should_restore_for_hotkey_event(hotkey_id, hotkey_id, false));
    }

    #[test]
    fn hotkey_toggle_action_hides_when_app_is_active_and_has_visible_window() {
        assert_eq!(
            build_hotkey_toggle_action(true, true),
            HotkeyToggleAction::Hide
        );
    }

    #[test]
    fn hotkey_toggle_action_restores_when_app_is_inactive_or_without_visible_window() {
        assert_eq!(
            build_hotkey_toggle_action(false, true),
            HotkeyToggleAction::Restore
        );
        assert_eq!(
            build_hotkey_toggle_action(true, false),
            HotkeyToggleAction::Restore
        );
    }
}
