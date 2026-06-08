//! Mica backdrop (Windows 11 22H2+), via GPUI's
//! [`WindowBackgroundAppearance::MicaBackdrop`](gpui::WindowBackgroundAppearance).

use gpui::{Window, WindowBackgroundAppearance};

/// The Mica backdrop requires Windows 11 22H2 (build 22621) or later,
/// GPUI silently ignores it on older builds.
const MIN_BUILD_NUMBER: u32 = 22621;

pub(crate) fn enable(window: &mut Window) -> bool {
    if windows_version::OsVersion::current().build < MIN_BUILD_NUMBER {
        return false;
    }

    window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop);
    true
}

pub(crate) fn disable(window: &mut Window) {
    window.set_background_appearance(WindowBackgroundAppearance::Opaque);
}
