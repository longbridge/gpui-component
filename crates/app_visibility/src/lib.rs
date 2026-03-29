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
    use cocoa::base::{BOOL, YES, id, nil};
    use cocoa::foundation::{NSString, NSUInteger};
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::OnceLock;

    static OBSERVER_CLASS: OnceLock<&'static Class> = OnceLock::new();
    static mut OBSERVER_INSTANCE: id = nil;

    pub fn register_activation_observer() -> Result<(), AppVisibilityError> {
        unsafe {
            let observer_class = if let Some(observer_class) = OBSERVER_CLASS.get() {
                *observer_class
            } else {
                let superclass = class!(NSObject);
                let mut decl = ClassDecl::new("OnetCliActivationObserver", superclass)
                    .ok_or(AppVisibilityError::ObserverClassInit)?;
                decl.add_method(
                    sel!(applicationDidBecomeActive:),
                    application_did_become_active as extern "C" fn(&Object, Sel, id),
                );
                let observer_class = decl.register();
                let _ = OBSERVER_CLASS.set(observer_class);
                observer_class
            };

            if OBSERVER_INSTANCE != nil {
                return Ok(());
            }

            let observer: id = msg_send![observer_class, new];
            if observer == nil {
                return Err(AppVisibilityError::ObserverInstanceInit);
            }

            let notification_center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
            let app = shared_application();
            let name = NSString::alloc(nil).init_str("NSApplicationDidBecomeActiveNotification");
            let _: () = msg_send![notification_center, addObserver: observer
                selector: sel!(applicationDidBecomeActive:)
                name: name
                object: app
            ];
            OBSERVER_INSTANCE = observer;

            Ok(())
        }
    }

    pub fn hide_main_window() -> Result<(), AppVisibilityError> {
        unsafe {
            hide_app(shared_application());
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
            let app = shared_application();
            let action = build_main_window_visibility_action(app_is_active(app), has_visible_window(app));

            match action {
                MainWindowVisibilityAction::Hide => hide_app(app),
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

    extern "C" fn application_did_become_active(_: &Object, _: Sel, _: id) {
        let _ = restore_main_window();
    }

    unsafe fn shared_application() -> id {
        msg_send![class!(NSApplication), sharedApplication]
    }

    unsafe fn restore_first_minimized_window() -> bool {
        let app = unsafe { shared_application() };
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
        let app = unsafe { shared_application() };
        let _: () = msg_send![app, unhide: nil];
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    }

    unsafe fn hide_app(app: id) {
        let _: () = msg_send![app, hide: nil];
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
        Err(AppVisibilityError::UnsupportedPlatform)
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
        restore_main_window()
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
    use super::AppVisibilityError;
    use raw_window_handle::RawWindowHandle;
    use std::sync::{Mutex, OnceLock};

    // FIX 1: 同 Windows 平台，用 wrapper 手动标记为 Send。
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
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn restore_main_window() -> Result<(), AppVisibilityError> {
        let _ = main_window_handle_slot()
            .lock()
            .expect("主窗口句柄锁不应中毒")
            .as_ref()
            .ok_or(AppVisibilityError::MainWindowHandleMissing)?;
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn toggle_main_window_visibility() -> Result<(), AppVisibilityError> {
        restore_main_window()
    }

    pub fn register_main_window_handle(window_handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
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
        Err(AppVisibilityError::UnsupportedPlatform)
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