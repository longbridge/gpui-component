//! Windows native menu implementation (Win32 popup menus).

use std::ffi::c_void;

use gpui::{App, Pixels, Point, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::SetCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    PostMessageW, SetForegroundWindow, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_TOPALIGN,
    TrackPopupMenuEx, WM_NULL,
};
use windows::core::PCWSTR;

use super::NativeMenuItem;

/// Show a native popup menu and dispatch the selected item's action.
///
/// The Win32 tracking loop (`TrackPopupMenuEx`) blocks, so — like macOS — it is
/// run from a foreground task to avoid re-entering GPUI while it is borrowed.
pub(super) fn popup(
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
        let Some(index) = run_menu(hwnd, &items, client_x, client_y) else {
            return;
        };
        let Some(NativeMenuItem::Item {
            action: Some(action),
            ..
        }) = items.get(index)
        else {
            return;
        };
        let action = action.boxed_clone();

        cx.update(move |app| {
            let _ = handle.update(app, move |_, window, app| {
                window.dispatch_action(action, app);
            });
        });
    })
    .detach();
}

/// Build and synchronously run the popup menu, returning the selected item index.
fn run_menu(hwnd: isize, items: &[NativeMenuItem], client_x: i32, client_y: i32) -> Option<usize> {
    let hwnd = HWND(hwnd as *mut c_void);

    // SAFETY: Win32 menu calls on a live window owned by the calling (main)
    // thread. The menu is destroyed before returning.
    unsafe {
        let hmenu = CreatePopupMenu().ok()?;

        for (index, item) in items.iter().enumerate() {
            match item {
                NativeMenuItem::Separator => {
                    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
                }
                NativeMenuItem::Item {
                    label,
                    disabled,
                    checked,
                    ..
                } => {
                    let mut flags = MF_STRING;
                    if *disabled {
                        flags |= MF_GRAYED;
                    }
                    if *checked {
                        flags |= MF_CHECKED;
                    }
                    let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                    // Item ids are 1-based; `TrackPopupMenuEx` returns 0 for "no
                    // selection", so reserve 0 and map back with `id - 1`.
                    let _ = AppendMenuW(hmenu, flags, index + 1, PCWSTR(wide.as_ptr()));
                }
            }
        }

        // Convert the window-relative (client) point to screen coordinates.
        let mut point = POINT {
            x: client_x,
            y: client_y,
        };
        let _ = ClientToScreen(hwnd, &mut point);

        // Required so the menu dismisses correctly when clicking elsewhere.
        let _ = SetForegroundWindow(hwnd);

        let flags = TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_NONOTIFY;
        let selected = TrackPopupMenuEx(hmenu, flags.0, point.x, point.y, hwnd, None);
        let _ = DestroyMenu(hmenu);

        // The menu's modal loop took over and cleared the global mouse capture
        // that GPUI set on mouse-down. Restore it so GPUI's matching mouse-up
        // `ReleaseCapture` succeeds instead of "failing" with GetLastError == 0
        // and logging a spurious "operation completed successfully" message.
        let _ = SetCapture(hwnd);
        // MSDN-recommended quirk so the window's message queue recovers cleanly
        // after `TrackPopupMenuEx`.
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));

        match selected.0 {
            id if id > 0 => Some((id - 1) as usize),
            _ => None,
        }
    }
}

/// Extract the Win32 `HWND` (as an `isize`) from the window's raw handle.
fn hwnd_ptr(window: &Window) -> Option<isize> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    Some(handle.hwnd.get())
}
