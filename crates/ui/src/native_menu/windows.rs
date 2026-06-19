//! Windows native menu implementation (Win32 popup menus).

use std::ffi::c_void;

use gpui::{Action, App, Pixels, Point, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::SetCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
    MF_STRING, PostMessageW, SetForegroundWindow, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_TOPALIGN, TrackPopupMenuEx, WM_NULL,
};
use windows::core::PCWSTR;

use super::NativeMenuItem;

/// Show a native popup menu and dispatch the selected item's action.
///
/// The Win32 tracking loop (`TrackPopupMenuEx`) blocks, so — like macOS — it is
/// run from a foreground task to avoid re-entering GPUI while it is borrowed.
pub(super) fn show(
    items: Vec<NativeMenuItem>,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(hwnd) = hwnd_ptr(window) else {
        return;
    };
    // `position` is logical pixels; Win32 wants physical pixels.
    let scale = window.scale_factor();
    let client_x = (f32::from(position.x) * scale).round() as i32;
    let client_y = (f32::from(position.y) * scale).round() as i32;
    // Inherent `Window::window_handle` (GPUI's `AnyWindowHandle`), not the
    // `raw_window_handle::HasWindowHandle` trait method in scope below.
    let handle = Window::window_handle(window);

    cx.spawn(async move |cx| {
        let Some(action) = run_menu(hwnd, &items, client_x, client_y) else {
            return;
        };
        cx.update(move |app| {
            let _ = handle.update(app, move |_, window, app| {
                window.dispatch_action(action, app);
            });
        });
    })
    .detach();
}

/// Build the menu (recursively, including submenus), show it, and return the
/// selected item's action.
fn run_menu(
    hwnd: isize,
    items: &[NativeMenuItem],
    client_x: i32,
    client_y: i32,
) -> Option<Box<dyn Action>> {
    let hwnd = HWND(hwnd as *mut c_void);

    // SAFETY: Win32 menu calls on a live window owned by the calling (main)
    // thread. The menu (and its submenus) is destroyed before returning.
    unsafe {
        let mut actions: Vec<&Box<dyn Action>> = Vec::new();
        let menu = build_menu(items, &mut actions)?;

        let mut point = POINT {
            x: client_x,
            y: client_y,
        };
        let _ = ClientToScreen(hwnd, &mut point);
        // Required so the menu dismisses correctly when clicking elsewhere.
        let _ = SetForegroundWindow(hwnd);

        let flags = TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_NONOTIFY;
        let selected = TrackPopupMenuEx(menu, flags.0, point.x, point.y, hwnd, None);
        // Destroying the top menu also destroys its attached submenus.
        let _ = DestroyMenu(menu);

        // The menu's modal loop cleared the capture GPUI set on mouse-down;
        // restore it so GPUI's mouse-up `ReleaseCapture` succeeds and doesn't
        // log a spurious "operation completed successfully" (GetLastError == 0).
        let _ = SetCapture(hwnd);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));

        // Ids are 1-based (0 means "no selection"); map back to `actions`.
        match selected.0 {
            id if id > 0 => actions.get((id - 1) as usize).map(|action| action.boxed_clone()),
            _ => None,
        }
    }
}

/// Recursively create an `HMENU`. Each actionable leaf gets a 1-based id equal
/// to its index in `actions` plus one, so the returned id maps back to its action.
///
/// # Safety
/// Win32 menu creation; the returned `HMENU` must be destroyed by the caller.
unsafe fn build_menu<'a>(
    items: &'a [NativeMenuItem],
    actions: &mut Vec<&'a Box<dyn Action>>,
) -> Option<HMENU> {
    let menu = unsafe { CreatePopupMenu() }.ok()?;

    for item in items {
        match item {
            NativeMenuItem::Separator => {
                let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };
            }
            NativeMenuItem::Item {
                label,
                disabled,
                checked,
                action,
            } => {
                let mut flags = MF_STRING;
                if *disabled {
                    flags |= MF_GRAYED;
                }
                if *checked {
                    flags |= MF_CHECKED;
                }
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                // Actionable, enabled items get an id; others use 0.
                let id = match action {
                    Some(action) if !*disabled => {
                        actions.push(action);
                        actions.len()
                    }
                    _ => 0,
                };
                let _ = unsafe { AppendMenuW(menu, flags, id, PCWSTR(wide.as_ptr())) };
            }
            NativeMenuItem::Submenu {
                label,
                disabled,
                items,
            } => {
                let Some(submenu) = (unsafe { build_menu(items, actions) }) else {
                    continue;
                };
                let mut flags = MF_POPUP;
                if *disabled {
                    flags |= MF_GRAYED;
                }
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                // For MF_POPUP, the id parameter is the submenu handle.
                let _ = unsafe {
                    AppendMenuW(menu, flags, submenu.0 as usize, PCWSTR(wide.as_ptr()))
                };
            }
        }
    }

    Some(menu)
}

/// Extract the Win32 `HWND` (as an `isize`) from the window's raw handle.
fn hwnd_ptr(window: &Window) -> Option<isize> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    Some(handle.hwnd.get())
}
