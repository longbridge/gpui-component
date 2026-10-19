//! GPUI Component bindings for applications driven by NSH.
//!
//! Experimental bindings for native controls driven by NSH signals.
//! NSH owns the application runtime and signals; these adapters render the
//! original GPUI Component controls. No GPUI Shell runtime is involved.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

use nsh_core::ui::Registry;
use nsh_platform_gpui::Components;

pub mod bridge;
#[path = "families/carousel.rs"]
mod carousel;
#[path = "families/collections.rs"]
mod collections;
#[path = "families/content.rs"]
mod content;
#[path = "families/conversation.rs"]
mod conversation;
#[path = "families/datetime.rs"]
mod datetime;
mod dialog_parts;
#[path = "families/display.rs"]
mod display;
#[path = "families/dock.rs"]
mod dock;
mod extra_controls;
pub mod host;
mod input;
#[path = "families/inputs.rs"]
mod inputs;
#[path = "families/layout.rs"]
mod layout;
mod menus;
mod notification;
mod overlay;
#[path = "families/questionnaire.rs"]
mod questionnaire;
mod registration;
#[path = "families/scrolling.rs"]
mod scrolling;
mod select;
mod slider;
mod surfaces;

/// NSH's GPUI backend and application loading APIs.
pub use nsh_platform_gpui as nsh;

/// Creates a native catalog to put in NSH's `AppOptions::components`.
///
/// Call `gpui_kit::init` before creating windows, and use
/// `gpui_kit::open_window` so Component overlays have their window host.
/// Typed exports and declarations come from [`ui_set`].
pub fn components() -> Components {
    native_catalog()
}

/// The NSH UI set imported as `nsh/ui/gpui`.
///
/// NSH generates its JavaScript exports and editor declarations from the same
/// Rust props and events that the native controls receive. Register this set
/// with NSH before creating an engine. The standalone hosts do this for you.
pub fn ui_set() -> Registry {
    let mut registry = Registry::ui_set("nsh/ui/gpui");
    register_typed(&mut registry);
    registry
}

fn register_typed(registry: &mut impl registration::BindingRegistry) {
    display::register_typed(registry);
    input::register_typed(registry);
    overlay::register_typed(registry);
    surfaces::register_typed(registry);
    extra_controls::register_typed(registry);
    dialog_parts::register_typed(registry);
    menus::register_typed(registry);
    scrolling::register_typed(registry);
    questionnaire::register_typed(registry);
    carousel::register_typed(registry);
    inputs::register_typed(registry);
    layout::register_typed(registry);
    content::register_typed(registry);
    datetime::register_typed(registry);
    dock::register_typed(registry);
    collections::register_typed(registry);
    conversation::register_typed(registry);
    select::register_typed(registry);
    slider::register_typed(registry);
    notification::register_typed(registry);
}

fn native_catalog() -> Components {
    let components = Components::default();
    let mut registry = Registry::ui_set("nsh/ui/gpui");
    register_typed(&mut (&components, &mut registry));
    components
}
