//! System glass background effect for windows, see
//! [`crate::WindowExt::set_window_glass`].
//!
//! Platform implementations live in submodules:
//!
//! - macOS 26+ : Liquid Glass via a native `NSGlassEffectView` ([`macos`]).
//! - Windows 11 22H2+ : Mica backdrop ([`windows`]).
//! - Other platforms (older systems, Linux): no-op fallback.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::{disable, enable};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use windows::{disable, enable};

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn enable(_window: &mut gpui::Window) -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn disable(_window: &mut gpui::Window) {}
