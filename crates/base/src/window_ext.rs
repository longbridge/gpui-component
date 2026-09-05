//! Platform-specific window configuration utilities.
//!
//! Provides cross-platform APIs to configure window behavior beyond what
//! [`WindowOptions`] offers:
//!
//! - **Skip taskbar** — hide the window from the taskbar / dock / alt-tab
//! - **Click-through** — mouse events pass through the window
//! - **Always on top** — window stays above other windows
//!
//! # Example
//!
//! ```no_run
//! use gpui_kit::base::window_ext;
//!
//! // After creating a window, configure it as an overlay:
//! window_ext::make_overlay(&window);
//! ```
//!
//! [`WindowOptions`]: gpui::WindowOptions

use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Window behavior configuration.
///
/// Construct with the builder methods and pass to [`configure`].
///
/// # Example
///
/// ```
/// use gpui_kit::base::window_ext::WindowBehavior;
///
/// let behavior = WindowBehavior::new()
///     .skip_taskbar(true)
///     .click_through(true)
///     .always_on_top(true);
/// ```
#[derive(Clone, Debug, Default)]
pub struct WindowBehavior {
    skip_taskbar: bool,
    click_through: bool,
    always_on_top: bool,
}

impl WindowBehavior {
    /// Create a new `WindowBehavior` with all options disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the window is hidden from the taskbar, dock, and alt-tab.
    pub fn is_skip_taskbar(&self) -> bool {
        self.skip_taskbar
    }

    /// Whether mouse events pass through the window.
    pub fn is_click_through(&self) -> bool {
        self.click_through
    }

    /// Whether the window stays above other windows.
    pub fn is_always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// Set whether to hide the window from the taskbar.
    pub fn with_skip_taskbar(mut self, skip: bool) -> Self {
        self.skip_taskbar = skip;
        self
    }

    /// Set whether mouse events pass through the window.
    pub fn with_click_through(mut self, click_through: bool) -> Self {
        self.click_through = click_through;
        self
    }

    /// Set whether the window should stay on top of other windows.
    pub fn with_always_on_top(mut self, on_top: bool) -> Self {
        self.always_on_top = on_top;
        self
    }
}

/// Apply a [`WindowBehavior`] configuration to a window.
///
/// Call this after the window has been created (e.g. inside the
/// `open_window` closure, or from a `cx.defer` callback).
///
/// # Example
///
/// ```no_run
/// use gpui_kit::base::window_ext::{self, WindowBehavior};
///
/// // Inside a window handler:
/// let behavior = WindowBehavior::new()
///     .with_skip_taskbar(true)
///     .with_always_on_top(true);
/// window_ext::configure(&window, &behavior);
/// ```
pub fn configure(window: &Window, behavior: &WindowBehavior) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    match handle.as_raw() {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(handle) => configure_macos(handle, behavior),
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(handle) => configure_windows(handle, behavior),
        #[cfg(target_os = "linux")]
        RawWindowHandle::Xlib(handle) => configure_xlib(window, handle, behavior),
        #[cfg(target_os = "linux")]
        RawWindowHandle::Wayland(_) => {
            // Wayland does not support these features at the client level.
        }
        _ => {}
    }
}

/// Convenience: configure as an overlay window (skip taskbar + click-through + always on top).
pub fn make_overlay(window: &Window) {
    configure(
        window,
        &WindowBehavior::new()
            .with_skip_taskbar(true)
            .with_click_through(true)
            .with_always_on_top(true),
    );
}

/// Convenience: configure as a floating window (skip taskbar + always on top, no click-through).
pub fn make_floating(window: &Window) {
    configure(
        window,
        &WindowBehavior::new()
            .with_skip_taskbar(true)
            .with_always_on_top(true),
    );
}

/// Convenience: enable click-through only.
pub fn set_click_through(window: &Window, enabled: bool) {
    configure(window, &WindowBehavior::new().with_click_through(enabled));
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn configure_macos(
    handle: raw_window_handle::AppKitWindowHandle,
    behavior: &WindowBehavior,
) {
    use objc2::runtime::NSObject;
    use objc2::{msg_send, ClassType};
    use objc2_app_kit::NSWindow;

    let ns_window_ptr = handle.ns_window as *mut NSObject;
    if ns_window_ptr.is_null() {
        return;
    }
    let ns_window: &NSWindow = unsafe { &*ns_window_ptr.cast::<NSWindow>() };

    unsafe {
        // Configure collection behavior (taskbar / alt-tab / spaces)
        if behavior.is_skip_taskbar() || behavior.is_always_on_top() {
            let mut collection_behavior: objc2::runtime::NSUInteger = 0;

            if behavior.is_skip_taskbar() {
                // NSWindowCollectionBehavior::CanJoinAllSpaces
                collection_behavior |= 1 << 0;
                // NSWindowCollectionBehavior::Stationary
                collection_behavior |= 1 << 6;
                // NSWindowCollectionBehavior::IgnoresCycle (hide from Alt-Tab)
                collection_behavior |= 1 << 7;
                // NSWindowCollectionBehavior::ExcludedFromWindowsMenu
                collection_behavior |= 1 << 8;
            }

            if behavior.is_always_on_top() {
                // NSWindowCollectionBehavior::CanJoinAllSpaces
                collection_behavior |= 1 << 0;
            }

            let _: () = msg_send![ns_window, setCollectionBehavior: collection_behavior];
        }

        // Configure window level (always on top)
        if behavior.is_always_on_top() {
            // NSFloatingWindowLevel = 3, use 5 to be above floating
            let _: () = msg_send![ns_window, setLevel: 5];
        }

        // Configure click-through
        if behavior.is_click_through() {
            let _: () = msg_send![ns_window, setIgnoresMouseEvents: true];
        }
    }
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn configure_windows(
    handle: raw_window_handle::Win32WindowHandle,
    behavior: &WindowBehavior,
) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::*;

    let hwnd = HWND(handle.hwnd as _);
    if hwnd.is_invalid() {
        return;
    }

    unsafe {
        // Build extended window styles
        let mut ex_style: WINDOW_EX_STYLE = WINDOW_EX_STYLE(0);

        if behavior.is_skip_taskbar() {
            // WS_EX_TOOLWINDOW: hide from taskbar
            ex_style |= WS_EX_TOOLWINDOW;
            // WS_EX_NOACTIVATE: don't steal focus
            ex_style |= WS_EX_NOACTIVATE;
        }

        if behavior.is_click_through() {
            // WS_EX_TRANSPARENT: click-through
            ex_style |= WS_EX_TRANSPARENT;
            // WS_EX_LAYERED: required for transparent styles
            ex_style |= WS_EX_LAYERED;
        }

        // Apply extended styles
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style.0 as i32);

        // Set z-order
        if behavior.is_always_on_top() {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        } else {
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Linux/X11 implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn configure_xlib(
    _window: &Window,
    handle: raw_window_handle::XlibWindowHandle,
    behavior: &WindowBehavior,
) {
    use x11_dl::xlib;

    // X11 requires a Display connection. We open a new one here because
    // GPUI does not expose its display handle.
    let xlib = match x11_dl::xlib::Xlib::open() {
        Ok(x) => x,
        Err(_) => return,
    };

    unsafe {
        let display = (xlib.XOpenDisplay)(std::ptr::null());
        if display.is_null() {
            return;
        }
        let window = handle.window as xlib::Window;

        // Helper: intern an atom
        let intern_atom = |name: &[u8]| -> xlib::Atom {
            (xlib.XInternAtom)(display, name.as_ptr() as *const i8, 0)
        };

        // Helper: set a single Atom property on the window
        let set_atom_property = |property: xlib::Atom, value: xlib::Atom| {
            (xlib.XChangeProperty)(
                display,
                window,
                property,
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                &value as *const _ as *const u8,
                1,
            );
        };

        // Set window type to DOCK to hide from taskbar/pager
        if behavior.is_skip_taskbar() {
            let atom_net_wm_window_type = intern_atom(b"_NET_WM_WINDOW_TYPE\0");
            let atom_dock = intern_atom(b"_NET_WM_WINDOW_TYPE_DOCK\0");
            set_atom_property(atom_net_wm_window_type, atom_dock);

            // Also set SKIP_TASKBAR state
            let atom_net_wm_state = intern_atom(b"_NET_WM_STATE\0");
            let atom_skip_taskbar = intern_atom(b"_NET_WM_STATE_SKIP_TASKBAR\0");
            set_atom_property(atom_net_wm_state, atom_skip_taskbar);
        }

        // Always on top
        if behavior.is_always_on_top() {
            let atom_net_wm_state = intern_atom(b"_NET_WM_STATE\0");
            let atom_above = intern_atom(b"_NET_WM_STATE_ABOVE\0");
            set_atom_property(atom_net_wm_state, atom_above);
        }

        // Click-through on X11 requires the Shape extension.
        // We set _NET_WM_WINDOW_TYPE_SPLASH as a hint, but true
        // click-through needs XShapeCombineRegion which requires
        // querying the shape extension first.
        if behavior.is_click_through() {
            let atom_net_wm_window_type = intern_atom(b"_NET_WM_WINDOW_TYPE\0");
            let atom_splash = intern_atom(b"_NET_WM_WINDOW_TYPE_SPLASH\0");
            set_atom_property(atom_net_wm_window_type, atom_splash);
        }

        (xlib.XFlush)(display);
        (xlib.XCloseDisplay)(display);
    }
}
