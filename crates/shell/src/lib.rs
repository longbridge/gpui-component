//! GPUI Shell — a Lua application runtime built on `gpui-base`.
//!
//! The host owns rendering, layout, text editing, virtualization and system
//! capabilities; Lua owns composition, presentation and business logic. See
//! `docs/research/gpui-shell.md` for the design this implements.

pub mod capability;
pub mod engine;
pub mod error;
pub mod materialize;
pub mod root;
pub mod runtime;
pub mod scope;
pub mod spec;
pub mod style;
pub mod theme;
pub mod value;
pub mod view;
pub mod watch;

pub use capability::Capabilities;
pub use engine::ShellRuntime;
pub use error::ShellError;
pub use root::{DialogOptions, SheetSide, ShellRoot, ToastLevel, ToastRequest};
pub use scope::ScopePhase;
pub use view::ScriptView;

use gpui::App;

/// Initializes the base layer, the shell's default semantic tokens, and the
/// style reflection table.
///
/// Must be called once at application startup, before any Lua runs.
pub fn init(cx: &mut App) {
    gpui_base::init(cx);
    theme::init(cx);
    style::init();
}
