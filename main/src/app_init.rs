use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code as HotkeyCode, HotKey, Modifiers as HotkeyModifiers},
};
use gpui::App;
use raw_window_handle::HasWindowHandle;
use std::sync::OnceLock;
use std::time::Duration;
/// 全局初始化标记（只初始化一次）
static VISIBILITY_INIT: OnceLock<()> = OnceLock::new();

/// 对外暴露：初始化窗口相关系统（visibility + hotkey）
pub fn init_window_systems(window: &impl HasWindowHandle, cx: &mut App) {
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
    system_hotkey::register(cx);

    // 标记完成
    let _ = VISIBILITY_INIT.set(());
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod system_hotkey {
    use super::*;
    use crate::app_init::HotkeyModifiers;
    use gpui::AsyncApp;
    use std::sync::{Mutex, OnceLock, mpsc};

    const HOTKEY_POLL_INTERVAL: Duration = Duration::from_millis(16);

    static HOTKEY_MANAGER: OnceLock<Mutex<GlobalHotKeyManager>> = OnceLock::new();
    static TOGGLE_HOTKEY_ID: OnceLock<u32> = OnceLock::new();
    static TOGGLE_REQUEST_TX: OnceLock<mpsc::Sender<()>> = OnceLock::new();

    pub fn register(cx: &mut App) {
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

        install_toggle_dispatcher(cx);

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
            if let Some(tx) = TOGGLE_REQUEST_TX.get() {
                if let Err(err) = tx.send(()) {
                    tracing::warn!("主窗口热键事件派发失败: {err}");
                }
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

    fn install_toggle_dispatcher(cx: &mut App) {
        if TOGGLE_REQUEST_TX.get().is_some() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        if TOGGLE_REQUEST_TX.set(tx).is_err() {
            return;
        }

        cx.spawn(move |cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                loop {
                    cx.background_executor().timer(HOTKEY_POLL_INTERVAL).await;
                    while rx.try_recv().is_ok() {
                        if let Err(err) = cx.update(|_| app_visibility::toggle()) {
                            tracing::warn!("主窗口系统级快捷键处理失败: {err:?}");
                        }
                    }
                }
            }
        })
        .detach();
    }
}
