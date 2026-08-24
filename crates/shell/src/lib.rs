//! GPUI Shell — a Lua application runtime built on `gpui-base`.
//!
//! The host owns rendering, layout, text editing, virtualization and system
//! capabilities; Lua owns composition, presentation and business logic. See
//! `docs/research/gpui-shell.md` for the design this implements.

pub mod capability;
pub mod engine;
pub mod entities;
pub mod error;
pub mod materialize;
pub mod root;
pub mod runtime;
pub mod scope;
pub mod spec;
pub mod style;
pub mod theme;
pub mod typings;
pub mod value;
pub mod view;
pub mod watch;

pub use capability::Capabilities;
pub use engine::ShellRuntime;
pub use error::ShellError;
pub use root::{DialogOptions, SheetSide, ShellRoot, ToastLevel, ToastRequest};
pub use scope::ScopePhase;
pub use view::ScriptView;

use std::path::PathBuf;

use gpui::App;

/// Grants an application its capabilities.
///
/// Nothing is permitted until this is called: a script gets no file, storage,
/// clipboard or process access by default (design doc §5.7). The host decides,
/// because only the host knows how much the code it is about to run is trusted.
#[cfg(feature = "quickjs")]
pub fn set_capabilities(capabilities: Capabilities) {
    engine::quickjs::host::set_capabilities(capabilities);
}

#[cfg(not(feature = "quickjs"))]
pub fn set_capabilities(_capabilities: Capabilities) {}

/// Points `gpui.store` at a directory.
///
/// Storage is per application, and the host chooses where that is — an
/// application cannot name its own storage location, or two applications could
/// collide on purpose.
#[cfg(feature = "quickjs")]
pub fn set_store_path(path: PathBuf) {
    engine::quickjs::host::set_store_path(path);
}

#[cfg(not(feature = "quickjs"))]
pub fn set_store_path(_path: PathBuf) {}

/// Relaxes the sandbox for a development session.
///
/// Restores `eval` and unfreezes the built-in prototypes, which a REPL needs
/// and a shipped application must not have.
#[cfg(feature = "quickjs")]
pub fn set_development_mode(enabled: bool) {
    engine::quickjs::sandbox::set_development_mode(enabled);
}

#[cfg(not(feature = "quickjs"))]
pub fn set_development_mode(_enabled: bool) {}

/// Initializes the base layer, the shell's default semantic tokens, and the
/// style reflection table.
///
/// Must be called once at application startup, before any Lua runs.
pub fn init(cx: &mut App) {
    gpui_base::init(cx);
    theme::init(cx);
    style::init();
}
