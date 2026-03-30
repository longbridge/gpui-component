use global_hotkey::{
    hotkey::{Code as HotkeyCode, HotKey, Modifiers as HotkeyModifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use gpui::{AnyWindowHandle, App, Window};
use std::sync::OnceLock;
use std::time::Duration;

/// 全局初始化标记（只初始化一次）
static VISIBILITY_INIT: OnceLock<()> = OnceLock::new();
static MAIN_WINDOW_HANDLE: OnceLock<AnyWindowHandle> = OnceLock::new();

/// 对外暴露：初始化窗口相关系统（visibility + hotkey）
pub fn init_window_systems(window: &Window, cx: &mut App) {
    if VISIBILITY_INIT.get().is_some() {
        tracing::debug!("window systems already initialized");
        return;
    }

    let _ = MAIN_WINDOW_HANDLE.set(window.window_handle());

    // 初始化 hotkey
    system_hotkey::register(cx);

    let _ = VISIBILITY_INIT.set(());
}

fn pick_toggle_target<T: Copy>(
    registered: Option<T>,
    stacked: Option<&[T]>,
    fallback: Option<T>,
) -> Option<T> {
    registered
        .or_else(|| stacked.and_then(|stack| stack.first().copied()))
        .or(fallback)
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
mod system_hotkey {
    use super::*;
    use gpui::{AppContext, AsyncApp, Window};
    use std::sync::{mpsc, OnceLock};

    const HOTKEY_POLL_INTERVAL: Duration = Duration::from_millis(16);

    static TOGGLE_HOTKEY_ID: OnceLock<u32> = OnceLock::new();
    static TOGGLE_REQUEST_TX: OnceLock<mpsc::Sender<()>> = OnceLock::new();
    static REGISTERED: OnceLock<()> = OnceLock::new();

    pub fn register(cx: &mut App) {
        // 防止重复注册
        if REGISTERED.set(()).is_err() {
            tracing::debug!("hotkey already registered");
            return;
        }

        install_toggle_dispatcher(cx);

        // ✅ 关键：局部创建（不能 static）
        let manager = GlobalHotKeyManager::new()
            .map_err(|err| {
                tracing::warn!("系统级热键管理器初始化失败: {err:?}");
            })
            .ok()
            .expect("GlobalHotKeyManager 初始化失败");

        let hotkey = build_toggle_hotkey();
        let hotkey_id = hotkey.id();

        if let Err(err) = manager.register(hotkey) {
            tracing::warn!("系统级热键注册失败: {err:?}");
            return;
        }

        let _ = TOGGLE_HOTKEY_ID.set(hotkey_id);

        GlobalHotKeyEvent::set_event_handler(Some(handle_hotkey_event));

        // ⚠️ 防止 drop（否则热键失效）
        std::mem::forget(manager);
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
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(HOTKEY_POLL_INTERVAL)
                        .await;

                    while rx.try_recv().is_ok() {
                        if let Err(err) = toggle_main_window(&mut cx) {
                            tracing::warn!("主窗口系统级快捷键处理失败: {err:?}");
                        }
                    }
                }
            }
        })
            .detach();
    }

    fn toggle_main_window(cx: &mut AsyncApp) -> anyhow::Result<()> {
        if let Some(window_handle) = resolve_toggle_target(cx) {
            let _ = cx.update_window(window_handle, |_, window: &mut Window, _| {
                if window.is_window_active() {
                    window.minimize_window();
                } else {
                    window.activate_window();
                }
            });
        }
        Ok(())
    }

    fn resolve_toggle_target(cx: &AsyncApp) -> Option<AnyWindowHandle> {
        cx.update(|app| {
            let window_stack = app.window_stack();
            pick_toggle_target(
                MAIN_WINDOW_HANDLE.get().copied(),
                window_stack.as_deref(),
                app.active_window(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_toggle_target_prefers_registered_window() {
        let stack = [2_u8, 3_u8];

        assert_eq!(
            pick_toggle_target(Some(1_u8), Some(&stack), Some(9_u8)),
            Some(1_u8)
        );
        assert_eq!(
            pick_toggle_target(None, Some(&stack), Some(9_u8)),
            Some(2_u8)
        );
        assert_eq!(pick_toggle_target(None, None, Some(9_u8)), Some(9_u8));
    }

    #[test]
    fn pick_toggle_target_skips_empty_window_stack() {
        let empty: [u8; 0] = [];
        assert_eq!(
            pick_toggle_target(None, Some(&empty), Some(7_u8)),
            Some(7_u8)
        );
    }
}