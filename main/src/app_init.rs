use raw_window_handle::HasWindowHandle;
use std::sync::OnceLock;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code as HotkeyCode, HotKey, Modifiers as HotkeyModifiers},
};
/// 全局初始化标记（只初始化一次）
static VISIBILITY_INIT: OnceLock<()> = OnceLock::new();

/// 对外暴露：初始化窗口相关系统（visibility + hotkey）
pub fn init_window_systems(window: &impl HasWindowHandle) {
    // 已初始化直接返回
    if VISIBILITY_INIT.get().is_some() {
        return;
    }

    // 获取 window handle
    let handle = match window.window_handle() {
        Ok(h) => h,
        Err(err) => {
            tracing::warn!("窗口句柄获取失败: {err:?}");
            return;
        }
    };

    // 初始化 visibility
    if let Err(err) = app_visibility::init(handle.as_raw()) {
        tracing::warn!("窗口可见性系统初始化失败: {err:?}");
        return;
    }

    // 初始化 hotkey（依赖 visibility）
    system_hotkey::register();

    // 标记完成
    let _ = VISIBILITY_INIT.set(());
}


#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod system_hotkey {
    use crate::app_init::HotkeyModifiers;
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

    fn dispatch_main_window_shortcut() -> Result<(), app_visibility::AppVisibilityError> {
        app_visibility::toggle()
    }
}