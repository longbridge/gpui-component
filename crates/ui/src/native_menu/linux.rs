//! Linux native menu implementation.

use gpui::{App, Pixels, Point, Window};

use super::NativeMenuItem;

/// Display a native popup menu and dispatch the selected item's action.
///
/// TODO(native-menu): implement via GTK (`gtk_menu_popup_at_pointer`) or the
/// relevant compositor protocol, then dispatch the action like the macOS
/// implementation. Currently a no-op placeholder so the crate builds on Linux.
pub(super) fn popup(
    _items: Vec<NativeMenuItem>,
    _position: Point<Pixels>,
    _window: &mut Window,
    _cx: &mut App,
) {
}
