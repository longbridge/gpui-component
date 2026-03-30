mod error;

pub use crate::error::*;
use raw_window_handle::RawWindowHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Visible,
    Hidden,
}

pub fn init(window_handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
    platform::init(window_handle)
}

pub fn hide() -> Result<(), AppVisibilityError> {
    platform::hide()
}

pub fn restore() -> Result<(), AppVisibilityError> {
    platform::restore()
}

pub fn toggle() -> Result<(), AppVisibilityError> {
    match platform::current_state()? {
        WindowState::Visible => hide(),
        WindowState::Hidden => restore(),
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{AppVisibilityError, WindowState};
    use raw_window_handle::RawWindowHandle;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, IsWindowVisible, SW_HIDE, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
    };

    static HWND_STORE: OnceLock<HWND> = OnceLock::new();

    fn hwnd() -> Result<HWND, AppVisibilityError> {
        HWND_STORE
            .get()
            .copied()
            .ok_or(AppVisibilityError::MainWindowHandleMissing)
    }

    pub fn init(handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
        let RawWindowHandle::Win32(h) = handle else {
            return Err(AppVisibilityError::UnsupportedWindowHandle);
        };

        let hwnd = HWND(h.hwnd.get() as *mut _);
        HWND_STORE
            .set(hwnd)
            .map_err(|_| AppVisibilityError::AlreadyInitialized)
    }

    pub fn current_state() -> Result<WindowState, AppVisibilityError> {
        let hwnd = hwnd()?;
        let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
        let iconic = unsafe { IsIconic(hwnd).as_bool() };

        Ok(if visible && !iconic {
            WindowState::Visible
        } else {
            WindowState::Hidden
        })
    }

    pub fn hide() -> Result<(), AppVisibilityError> {
        let hwnd = hwnd()?;
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
        Ok(())
    }

    pub fn restore() -> Result<(), AppVisibilityError> {
        let hwnd = hwnd()?;

        let is_visible = unsafe { IsWindowVisible(hwnd).as_bool() };
        let is_iconic = unsafe { IsIconic(hwnd).as_bool() };

        unsafe {
            if !is_visible {
                ShowWindow(hwnd, SW_SHOW);
            }
            if is_iconic {
                ShowWindow(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd);
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{AppVisibilityError, WindowState};
    use raw_window_handle::RawWindowHandle;
    use std::sync::OnceLock;

    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    static INITED: OnceLock<()> = OnceLock::new();

    pub fn init(_: RawWindowHandle) -> Result<(), AppVisibilityError> {
        INITED
            .set(())
            .map_err(|_| AppVisibilityError::AlreadyInitialized)
    }

    pub fn current_state() -> Result<WindowState, AppVisibilityError> {
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let windows: *mut AnyObject = msg_send![app, windows];
            let count: usize = msg_send![windows, count];

            for i in 0..count {
                let w: *mut AnyObject = msg_send![windows, objectAtIndex: i];

                let visible: bool = msg_send![w, isVisible];
                let mini: bool = msg_send![w, isMiniaturized];

                if visible && !mini {
                    return Ok(WindowState::Visible);
                }
            }
        }

        Ok(WindowState::Hidden)
    }

    pub fn hide() -> Result<(), AppVisibilityError> {
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![app, hide: std::ptr::null::<AnyObject>()];
        }
        Ok(())
    }

    pub fn restore() -> Result<(), AppVisibilityError> {
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let windows: *mut AnyObject = msg_send![app, windows];
            let count: usize = msg_send![windows, count];

            for i in 0..count {
                let w: *mut AnyObject = msg_send![windows, objectAtIndex: i];
                let mini: bool = msg_send![w, isMiniaturized];

                if mini {
                    let _: () = msg_send![w, deminiaturize: std::ptr::null::<AnyObject>()];
                    let _: () = msg_send![w, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];
                    let _: () = msg_send![app, activateIgnoringOtherApps: true];
                    return Ok(());
                }
            }

            let _: () = msg_send![app, unhide: std::ptr::null::<AnyObject>()];
            let _: () = msg_send![app, activateIgnoringOtherApps: true];
        }

        Ok(())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use super::{AppVisibilityError, WindowState};
    use raw_window_handle::RawWindowHandle;
    use std::sync::{Mutex, OnceLock};

    use std::{ffi::CString, mem, ptr};

    use x11::xlib::*;

    struct SendableWindow(Window);
    unsafe impl Send for SendableWindow {}

    static WINDOW: OnceLock<Mutex<Option<SendableWindow>>> = OnceLock::new();

    fn window() -> Result<Window, AppVisibilityError> {
        let guard = WINDOW.get_or_init(|| Mutex::new(None)).lock().unwrap();

        guard
            .as_ref()
            .map(|w| w.0)
            .ok_or(AppVisibilityError::MainWindowHandleMissing)
    }

    pub fn init(handle: RawWindowHandle) -> Result<(), AppVisibilityError> {
        let window = match handle {
            RawWindowHandle::Xlib(h) => h.window,
            RawWindowHandle::Xcb(h) => h.window.get().into(),
            _ => return Err(AppVisibilityError::UnsupportedWindowHandle),
        };

        *WINDOW.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(SendableWindow(window));

        Ok(())
    }

    struct DisplayGuard(*mut Display);

    impl DisplayGuard {
        fn open() -> Result<Self, AppVisibilityError> {
            let d = unsafe { XOpenDisplay(ptr::null()) };
            if d.is_null() {
                Err(AppVisibilityError::X11DisplayOpenFailed)
            } else {
                Ok(Self(d))
            }
        }
    }

    impl Drop for DisplayGuard {
        fn drop(&mut self) {
            unsafe { XCloseDisplay(self.0) };
        }
    }

    fn is_visible(dpy: *mut Display, win: Window) -> bool {
        unsafe {
            let mut attrs: XWindowAttributes = mem::zeroed();
            if XGetWindowAttributes(dpy, win, &mut attrs) == 0 {
                return false;
            }
            attrs.map_state == 2
        }
    }

    unsafe fn focus(dpy: *mut Display, win: Window) {
        let atom = CString::new("_NET_ACTIVE_WINDOW").unwrap();
        let atom = XInternAtom(dpy, atom.as_ptr(), 0);
        if atom == 0 {
            return;
        }

        let root = XDefaultRootWindow(dpy);

        let mut ev: XEvent = mem::zeroed();
        let mut data: ClientMessageData = mem::zeroed();

        let arr = [2, 0, 0, 0, 0];
        ptr::copy_nonoverlapping(arr.as_ptr(), data.as_mut() as *mut i64, 5);

        let msg = XClientMessageEvent {
            type_: ClientMessage,
            serial: 0,
            send_event: 1,
            display: dpy,
            window: win,
            message_type: atom,
            format: 32,
            data,
        };

        ptr::copy_nonoverlapping(&msg, &mut ev as *mut _ as *mut _, 1);

        XSendEvent(
            dpy,
            root,
            0,
            SubstructureNotifyMask | SubstructureRedirectMask,
            &mut ev,
        );

        XFlush(dpy);
    }

    pub fn current_state() -> Result<WindowState, AppVisibilityError> {
        let win = window()?;
        let dpy = DisplayGuard::open()?;

        Ok(if is_visible(dpy.0, win) {
            WindowState::Visible
        } else {
            WindowState::Hidden
        })
    }

    pub fn hide() -> Result<(), AppVisibilityError> {
        let win = window()?;
        let dpy = DisplayGuard::open()?;

        unsafe {
            XWithdrawWindow(dpy.0, win, XDefaultScreen(dpy.0));
            XFlush(dpy.0);
        }

        Ok(())
    }

    pub fn restore() -> Result<(), AppVisibilityError> {
        let win = window()?;
        let dpy = DisplayGuard::open()?;

        unsafe {
            XMapWindow(dpy.0, win);
            focus(dpy.0, win);
        }

        Ok(())
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
mod platform {
    use super::{AppVisibilityError, WindowState};
    use raw_window_handle::RawWindowHandle;

    pub fn init(_: RawWindowHandle) -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn current_state() -> Result<WindowState, AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn hide() -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }

    pub fn restore() -> Result<(), AppVisibilityError> {
        Err(AppVisibilityError::UnsupportedPlatform)
    }
}
