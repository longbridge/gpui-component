//! GPUI Base controls for applications whose state and composition live in NSH.
//!
//! Depend on this crate for the unstyled Base layer. The separate
//! `gpui-kit-nsh` crate continues to expose the styled Component library.
//! Both hosts use NSH's existing runtime and GPUI backend.
#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

use nsh_core::ui::Registry;
use nsh_platform_gpui::Components;

pub mod bridge;
mod controls;
pub mod host;
mod registration;

/// NSH's existing GPUI backend, typed components and application loading APIs.
pub use nsh_platform_gpui as nsh;

/// Generates the JavaScript exports and declarations imported as `nsh/ui/base`.
/// Register this set with NSH before creating an engine; the standalone hosts
/// and CLI do so automatically.
pub fn ui_set() -> Registry {
    let mut registry = Registry::ui_set("nsh/ui/base");
    controls::register(&mut registry);
    registry
}

/// Creates the real native Base controls to install in `AppOptions::components`.
/// Initialize Kit and create windows with `gpui_kit::open_window`; with this
/// crate's feature configuration those operations initialize and host Base.
pub fn components() -> Components {
    let components = Components::default();
    let mut registry = Registry::ui_set("nsh/ui/base");
    controls::register(&mut (&components, &mut registry));
    components
}
