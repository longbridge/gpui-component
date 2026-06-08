//! Windows native menu implementation (Win32 popup menus).

use gpui::{App, Pixels, Point, Window};

use super::NativeMenuItem;

/// Display a native popup menu and dispatch the selected item's action.
///
/// TODO(native-menu): implement via `CreatePopupMenu` / `AppendMenuW` /
/// `TrackPopupMenuEx` with `TPM_RETURNCMD`, using the `HWND` from the window's
/// raw handle, then dispatch the action like the macOS implementation.
/// Currently a no-op placeholder so the crate builds on Windows.
pub(super) fn popup(
    _items: Vec<NativeMenuItem>,
    _position: Point<Pixels>,
    _window: &mut Window,
    _cx: &mut App,
) {
}
