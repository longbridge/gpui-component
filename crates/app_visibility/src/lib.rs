mod error;

pub use error::AppVisibilityError;
use raw_window_handle::RawWindowHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainWindowVisibilityAction {
    Hide,
    Restore,
}

pub fn build_main_window_visibility_action(
    app_is_active: bool,
    has_visible_window: bool,
) -> MainWindowVisibilityAction {
    if app_is_active && has_visible_window {
        MainWindowVisibilityAction::Hide
    } else {
        MainWindowVisibilityAction::Restore
    }
}

pub fn register_activation_observer() -> Result<(), AppVisibilityError> {
    platform::register_activation_observer()
}

pub fn hide_main_window() -> Result<(), AppVisibilityError> {
    platform::hide_main_window()
}

pub fn restore_main_window() -> Result<(), AppVisibilityError> {
    platform::restore_main_window()
}

pub fn toggle_main_window_visibility() -> Result<(), AppVisibilityError> {
    platform::toggle_main_window_visibility()
}

pub fn register_main_window_handle(window_handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
    platform::register_main_window_handle(window_handle)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{AppVisibilityError, MainWindowVisibilityAction, build_main_window_visibility_action};
    use raw_window_handle::RawWindowHandle;
    use std::sync::OnceLock;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send, sel};
    use objc2_foundation::NSString;

    static OBSERVER_REGISTERED: OnceLock<()> = OnceLock::new();

    pub fn register_activation_observer() -> Result<(), AppVisibilityError> {
        if OBSERVER_REGISTERED.get().is_some() {
            return Ok(());
        }

        unsafe {
            let notification_center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let name = NSString::from_str("NSApplicationDidBecomeActiveNotification");

            let _: () = msg_send![notification_center, addObserver: app, selector: sel!(unhide:), name: name.as_ref() as &AnyObject, object: app];
        }

        let _ = OBSERVER_REGISTERED.set(());
        Ok(())
    }

    pub fn hide_main_window() -> Result<(), AppVisibilityError> {
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, hide: std::ptr::null::<AnyObject>()];
        }
        Ok(())
    }

    pub fn restore_main_window() -> Result<(), AppVisibilityError> {
        unsafe {
            if !restore_first_minimized_window() {
                activate_app();
            }
        }
        Ok(())
    }

    pub fn toggle_main_window_visibility() -> Result<(), AppVisibilityError> {
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let is_active: bool = msg_send![app, isActive];
            let has_visible = has_visible_window();

            let action = build_main_window_visibility_action(is_active, has_visible);
            match action {
                MainWindowVisibilityAction::Hide => {
                    let _: () = msg_send![app, hide: std::ptr::null::<AnyObject>()];
                }
                MainWindowVisibilityAction::Restore => {
                    if !restore_first_minimized_window() {
                        activate_app();
                    }
                }
            }
        }
        Ok(())
    }

    pub fn register_main_window_handle(_: RawWindowHandle) -> Result<(), AppVisibilityError> {
        Ok(())
    }

    unsafe fn restore_first_minimized_window() -> bool {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let windows: *mut AnyObject = msg_send![app, windows];
        let count: usize = msg_send![windows, count];

        for index in 0..count {
            let window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
            let is_miniaturized: bool = msg_send![window, isMiniaturized];

            if is_miniaturized {
                let _: () = msg_send![app, unhide: std::ptr::null::<AnyObject>()];
                let _: () = msg_send![window, deminiaturize: std::ptr::null::<AnyObject>()];
                let _: () = msg_send![window, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];
                let _: () = msg_send![app, activateIgnoringOtherApps: true];
                return true;
            }
        }
        false
    }

    unsafe fn has_visible_window() -> bool {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let windows: *mut AnyObject = msg_send![app, windows];
        let count: usize = msg_send![windows, count];

        for index in 0..count {
            let window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
            let is_miniaturized: bool = msg_send![window, isMiniaturized];
            if !is_miniaturized {
                return true;
            }
        }
        false
    }

    unsafe fn activate_app() {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, unhide: std::ptr::null::<AnyObject>()];
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::AppVisibilityError;
    use raw_window_handle::RawWindowHandle;
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindowVisible, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
    };

    // FIX 1: RawWindowHandle 不实现 Send，用 wrapper 手动标记为 Send。
    // 安全性：我们只在主线程注册句柄，且仅通过 Mutex 访问，保证了互斥。
    #[allow(dead_code)]
    struct SendableWindowHandle(RawWindowHandle);
    unsafe impl Send for SendableWindowHandle {}

    static MAIN_WINDOW_HANDLE: OnceLock<Mutex<Option<SendableWindowHandle>>> = OnceLock::new();

    fn main_window_handle_slot() -> &'static Mutex<Option<SendableWindowHandle>> {
        MAIN_WINDOW_HANDLE.get_or_init(|| Mutex::new(None))
    }

    pub fn register_activation_observer() -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn hide_main_window() -> Result<(), AppVisibilityError> {
        let guard = main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒");

        let stored_handle = guard
            .as_ref()
            .ok_or(AppVisibilityError::MainWindowHandleMissing)?
            .0;

        let RawWindowHandle::Win32(handle) = stored_handle else {
            return Err(AppVisibilityError::UnsupportedWindowHandle);
        };

        let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
        unsafe {
            let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_HIDE);
        }
        Ok(())
    }

    pub fn restore_main_window() -> Result<(), AppVisibilityError> {
        let guard = main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒");

        let stored_handle = guard
            .as_ref()
            .ok_or(AppVisibilityError::MainWindowHandleMissing)?
            .0;

        let RawWindowHandle::Win32(handle) = stored_handle else {
            return Err(AppVisibilityError::UnsupportedWindowHandle);
        };

        // FIX 2: HWND 期望 *mut c_void，但 handle.hwnd.get() 返回 isize，需要转型。
        let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
        let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
        let is_iconic = unsafe { IsIconic(hwnd).as_bool() };

        if !is_visible {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
            }
            return Ok(());
        }

        if is_iconic {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
        }

        Ok(())
    }

    pub fn toggle_main_window_visibility() -> Result<(), AppVisibilityError> {
        let guard = main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒");

        let stored_handle = guard
            .as_ref()
            .ok_or(AppVisibilityError::MainWindowHandleMissing)?
            .0;

        let RawWindowHandle::Win32(handle) = stored_handle else {
            return Err(AppVisibilityError::UnsupportedWindowHandle);
        };

        let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
        let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };

        if is_visible {
            // 隐藏由 GPUI 处理
        } else {
            restore_main_window()?;
        }
        Ok(())
    }

    pub fn register_main_window_handle(window_handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
        *main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒") = Some(SendableWindowHandle(window_handle));
        Ok(())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use super::{AppVisibilityError, MainWindowVisibilityAction};
    use raw_window_handle::RawWindowHandle;
    use std::sync::{Mutex, OnceLock};
    use x11::xlib::{
        Display, Window, XCloseDisplay, XDefaultRootWindow, XFlush, XGetWindowAttributes,
        XInternAtom, XMapWindow, XOpenDisplay, XSendEvent, XWithdrawWindow, XWindowAttributes,
        ClientMessage, SubstructureNotifyMask, SubstructureRedirectMask,
        XEvent, XClientMessageEvent,
    };
    use std::ffi::CString;
    use std::mem;
    use std::ptr;

    // RawWindowHandle 不实现 Send，用 wrapper 手动标记。
    // 安全性：仅通过 Mutex 访问，保证互斥。
    #[allow(dead_code)]
    struct SendableWindowHandle(RawWindowHandle);
    unsafe impl Send for SendableWindowHandle {}

    static MAIN_WINDOW_HANDLE: OnceLock<Mutex<Option<SendableWindowHandle>>> = OnceLock::new();

    fn main_window_handle_slot() -> &'static Mutex<Option<SendableWindowHandle>> {
        MAIN_WINDOW_HANDLE.get_or_init(|| Mutex::new(None))
    }

    /// 打开一个临时的 Display 连接并在操作完成后关闭。
    /// X11 的 Display 连接不能跨线程共享，因此每次操作都单独开启。
    struct DisplayGuard(*mut Display);

    impl DisplayGuard {
        fn open() -> Result<Self, AppVisibilityError> {
            let display = unsafe { XOpenDisplay(ptr::null()) };
            if display.is_null() {
                Err(AppVisibilityError::X11DisplayOpenFailed)
            } else {
                Ok(DisplayGuard(display))
            }
        }

        fn as_ptr(&self) -> *mut Display {
            self.0
        }
    }

    impl Drop for DisplayGuard {
        fn drop(&mut self) {
            unsafe { XCloseDisplay(self.0) };
        }
    }

    /// 从已注册的 RawWindowHandle 中取出 X11 Window ID。
    fn get_xlib_window() -> Result<Window, AppVisibilityError> {
        let guard = main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒");

        let handle = guard
            .as_ref()
            .ok_or(AppVisibilityError::MainWindowHandleMissing)?
            .0;

        match handle {
            RawWindowHandle::Xlib(h) => Ok(h.window),
            _ => Err(AppVisibilityError::UnsupportedWindowHandle),
        }
    }

    /// 查询窗口当前是否处于可见（mapped）状态。
    /// XGetWindowAttributes 返回的 map_state：
    ///   IsUnmapped(0)       —— 已隐藏（withdraw 后）
    ///   IsUnviewable(1)     —— 已映射但父窗口不可见
    ///   IsViewable(2)       —— 正常显示
    fn is_window_viewable(display: *mut Display, window: Window) -> bool {
        unsafe {
            let mut attrs: XWindowAttributes = mem::zeroed();
            if XGetWindowAttributes(display, window, &mut attrs) == 0 {
                return false;
            }
            // IsViewable == 2
            attrs.map_state == 2
        }
    }

    /// 通过发送 _NET_ACTIVE_WINDOW 客户端消息请求窗口管理器将窗口置前。
    /// 这是 EWMH 标准方式，主流合规 WM（KWin、Mutter、Openbox 等）均支持。
    unsafe fn request_wm_focus(display: *mut Display, window: Window) {
        let atom_name = CString::new("_NET_ACTIVE_WINDOW").unwrap();
        let net_active_window = XInternAtom(display, atom_name.as_ptr(), 0);
        if net_active_window == 0 {
            // WM 不支持 EWMH，直接 XMapWindow 即可，不发消息
            return;
        }

        let root = XDefaultRootWindow(display);

        let mut event: XEvent = mem::zeroed();
        let cm = XClientMessageEvent {
            type_: ClientMessage,
            serial: 0,
            send_event: 1,
            display,
            window,
            message_type: net_active_window,
            format: 32,
            data: {
                let mut d: x11::xlib::ClientMessageData = mem::zeroed();
                // data.l[0] = 2 表示来自应用程序的激活请求（非用户直接操作）
                // data.l[1] = CurrentTime（0）
                // data.l[2] = 0（无当前活跃窗口）
                let arr: [i64; 5] = [2, 0, 0, 0, 0];
                std::ptr::copy_nonoverlapping(
                    arr.as_ptr(),
                    d.as_mut() as *mut i64,
                    5,
                );
                d
            },
        };
        std::ptr::copy_nonoverlapping(
            &cm as *const XClientMessageEvent,
            &mut event as *mut XEvent as *mut XClientMessageEvent,
            1,
        );

        XSendEvent(
            display,
            root,
            0,
            SubstructureNotifyMask | SubstructureRedirectMask,
            &mut event,
        );
        XFlush(display);
    }

    // ── 公开接口 ──────────────────────────────────────────────────────────────

    pub fn register_activation_observer() -> Result<(), AppVisibilityError> {
        // X11 没有类似 macOS NSApplicationDidBecomeActive 的全局激活通知机制，
        // 通常由上层（如 tray / global hotkey）直接调用 toggle_main_window_visibility。
        Ok(())
    }

    pub fn hide_main_window() -> Result<(), AppVisibilityError> {
        let window = get_xlib_window()?;
        let dpy = DisplayGuard::open()?;

        unsafe {
            // XWithdrawWindow 发送 UnmapNotify 并撤销 WM 装饰，是最干净的"隐藏"方式。
            // 第三个参数为 screen number，通常为 0。
            XWithdrawWindow(dpy.as_ptr(), window, 0);
            XFlush(dpy.as_ptr());
        }

        Ok(())
    }

    pub fn restore_main_window() -> Result<(), AppVisibilityError> {
        let window = get_xlib_window()?;
        let dpy = DisplayGuard::open()?;

        unsafe {
            // XMapWindow 重新映射窗口（对应 withdraw 后的恢复）。
            XMapWindow(dpy.as_ptr(), window);
            // 再通过 _NET_ACTIVE_WINDOW 消息请求 WM 将其置前并给予焦点。
            request_wm_focus(dpy.as_ptr(), window);
        }

        Ok(())
    }

    pub fn toggle_main_window_visibility() -> Result<(), AppVisibilityError> {
        let window = get_xlib_window()?;
        let dpy = DisplayGuard::open()?;

        // 窗口当前是否可见决定动作
        let visible = is_window_viewable(dpy.as_ptr(), window);
        let action = if visible {
            MainWindowVisibilityAction::Hide
        } else {
            MainWindowVisibilityAction::Restore
        };

        match action {
            MainWindowVisibilityAction::Hide => unsafe {
                XWithdrawWindow(dpy.as_ptr(), window, 0);
                XFlush(dpy.as_ptr());
            },
            MainWindowVisibilityAction::Restore => unsafe {
                XMapWindow(dpy.as_ptr(), window);
                request_wm_focus(dpy.as_ptr(), window);
            },
        }

        Ok(())
    }

    pub fn register_main_window_handle(window_handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
        // 仅接受 Xlib 句柄；xcb 句柄可通过 xcb_window_t 转换，但此处不做处理。
        match window_handle {
            RawWindowHandle::Xlib(_) => {}
            _ => return Err(AppVisibilityError::UnsupportedWindowHandle),
        }
        *main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒") = Some(SendableWindowHandle(window_handle));
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
mod platform {
    use super::AppVisibilityError;
    use raw_window_handle::RawWindowHandle;

    pub fn register_activation_observer() -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn hide_main_window() -> Result<(), AppVisibilityError> {
        let window = get_xlib_window()?;
        let dpy = DisplayGuard::open()?;

        unsafe {
            XWithdrawWindow(dpy.as_ptr(), window, 0);
            XFlush(dpy.as_ptr());
        }

        Ok(())
    }

    pub fn restore_main_window() -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn toggle_main_window_visibility() -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn register_main_window_handle(_: RawWindowHandle) -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::{MainWindowVisibilityAction, build_main_window_visibility_action};

    #[test]
    fn hides_main_window_when_app_is_active_and_has_visible_window() {
        assert_eq!(
            build_main_window_visibility_action(true, true),
            MainWindowVisibilityAction::Hide
        );
    }

    #[test]
    fn restores_main_window_when_app_is_inactive_or_window_is_hidden() {
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