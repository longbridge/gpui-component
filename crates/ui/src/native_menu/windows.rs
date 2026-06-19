//! Windows native menu implementation (Win32 popup menus).

use std::ffi::c_void;

use gpui::{Action, App, Pixels, Point, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, DeleteObject, HBITMAP, HGDIOBJ};
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateBitmapFromFile, GdipCreateHBITMAPFromBitmap, GdipDisposeImage, GdipGetImageThumbnail,
    GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpBitmap, GpImage,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, MENUITEMINFOW, MF_CHECKED, MF_GRAYED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MIIM_BITMAP, PostMessageW, SetForegroundWindow,
    SetMenuItemInfoW, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_TOPALIGN, TrackPopupMenuEx,
    WM_NULL,
};
use windows::core::PCWSTR;

use super::NativeMenuItem;

/// Side length (in points) menu item images are scaled to.
const MENU_IMAGE_SIZE: u32 = 16;

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
        // Start GDI+ so item images can be loaded into bitmaps. If it fails, the menu is still
        // built (images are skipped).
        let gdiplus = GdiplusSession::start();
        let mut actions: Vec<&Box<dyn Action>> = Vec::new();

        // Bitmaps attached to menu items must outlive the menu; freed below.
        let mut bitmaps: Vec<HBITMAP> = Vec::new();
        let menu = build_menu(items, &mut actions, &mut bitmaps)?;

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

        // The menu no longer references the bitmaps, so they can be freed.
        for bitmap in &bitmaps {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
        }
        drop(gdiplus);

        // The menu's modal loop cleared the capture GPUI set on mouse-down;
        // restore it so GPUI's mouse-up `ReleaseCapture` succeeds and doesn't
        // log a spurious "operation completed successfully" (GetLastError == 0).
        let _ = SetCapture(hwnd);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));

        // Ids are 1-based (0 means "no selection"); map back to `actions`.
        match selected.0 {
            id if id > 0 => actions
                .get((id - 1) as usize)
                .map(|action| action.boxed_clone()),
            _ => None,
        }
    }
}

/// Recursively create an `HMENU`. Each actionable leaf gets a 1-based id equal
/// to its index in `actions` plus one, so the returned id maps back to its action.
///
/// Any bitmaps created for item images are pushed onto `bitmaps`; the caller must free them after
/// destroying the menu with `DeleteObject`.
/// # Safety
/// Win32 menu creation; the returned `HMENU` must be destroyed by the caller.
unsafe fn build_menu<'a>(
    items: &'a [NativeMenuItem],
    actions: &mut Vec<&'a Box<dyn Action>>,
    bitmaps: &mut Vec<HBITMAP>,
) -> Option<HMENU> {
    let menu = unsafe { CreatePopupMenu() }.ok()?;

    // 0-based position of the next item appended, used to attach bitmaps by position (separators
    // and submenus advance it too).
    let mut position: u32 = 0;
    for item in items {
        match item {
            NativeMenuItem::Separator => {
                let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };
                position += 1;
            }
            NativeMenuItem::Item {
                label,
                disabled,
                checked,
                image,
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
                if let Some(image) = image {
                    if let Some(bitmap) = unsafe { load_hbitmap(image) } {
                        let info = MENUITEMINFOW {
                            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                            fMask: MIIM_BITMAP,
                            hbmpItem: bitmap,
                            ..Default::default()
                        };
                        let _ = unsafe { SetMenuItemInfoW(menu, position, true, &info) };
                        bitmaps.push(bitmap);
                    }
                }
                position += 1;
            }
            NativeMenuItem::Submenu {
                label,
                disabled,
                items,
            } => {
                let Some(submenu) = (unsafe { build_menu(items, actions, bitmaps) }) else {
                    continue;
                };
                let mut flags = MF_POPUP;
                if *disabled {
                    flags |= MF_GRAYED;
                }
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                // For MF_POPUP, the id parameter is the submenu handle.
                let _ =
                    unsafe { AppendMenuW(menu, flags, submenu.0 as usize, PCWSTR(wide.as_ptr())) };
                position += 1;
            }
        }
    }

    Some(menu)
}

/// RAII guard for a GDI+ session (`GdiplusStartup` / `GdiplusShutdown`).
///
/// Loading image files into bitmaps requires GDI+ to be initialized. A `None` token means startup
/// failed; image loading is then skipped gracefully.
struct GdiplusSession {
    token: usize,
}

impl GdiplusSession {
    /// Start GDI+. Returns a guard whose `Drop` calls `GdiplusShutdown`.
    unsafe fn start() -> Option<Self> {
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: 0,
            SuppressBackgroundThread: BOOL(0),
            SuppressExternalCodecs: BOOL(0),
        };

        let mut token: usize = 0;
        let status = unsafe { GdiplusStartup(&mut token, &input, std::ptr::null_mut()) };
        if status.0 == 0 {
            Some(Self { token })
        } else {
            None
        }
    }
}

impl Drop for GdiplusSession {
    fn drop(&mut self) {
        unsafe { GdiplusShutdown(self.token) };
    }
}

/// Load an image file into an `HBITMAP` via GDI+ (supports BMP, PNG, JPEG, ...). SVG is not supported.
///
/// This image is scaled to [`MENU_IMAGE_SIZE`] so it doesn't overflow the menu row. Returns `None`
/// if the file can't be read or decoded. GDI+ must already be initialized (see [`GdiplusSession`]).
/// The returned bitmap must be freed with `DeleteObject`.
///
/// # Safety
/// Calls GDI+ flat APIs; the returned handle is owned by the caller.
unsafe fn load_hbitmap(path: &str) -> Option<HBITMAP> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut gp_bitmap: *mut GpBitmap = std::ptr::null_mut();
    let status = unsafe { GdipCreateBitmapFromFile(PCWSTR(wide.as_ptr()), &mut gp_bitmap) };
    if status.0 != 0 || gp_bitmap.is_null() {
        return None;
    }

    // Scale to a menu icon sized thumbnail; GDI+ does not resize on display.
    let mut thumb: *mut GpImage = std::ptr::null_mut();
    let status = unsafe {
        GdipGetImageThumbnail(
            gp_bitmap.cast(),
            MENU_IMAGE_SIZE,
            MENU_IMAGE_SIZE,
            &mut thumb,
            0,
            std::ptr::null_mut(),
        )
    };

    unsafe { GdipDisposeImage(gp_bitmap.cast()) };
    if status.0 != 0 || thumb.is_null() {
        return None;
    }

    let mut hbitmap = HBITMAP::default();
    // ARGB background 0 (fully transparent) for the alpha conversion.
    let status = unsafe { GdipCreateHBITMAPFromBitmap(thumb.cast(), &mut hbitmap, 0) };

    unsafe { GdipDisposeImage(thumb) };

    if status.0 != 0 || hbitmap.is_invalid() {
        None
    } else {
        Some(hbitmap)
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
