//! GPUI Shell — a scriptable application runtime built on `gpui-base`.
//!
//! The host owns rendering, layout, text editing, virtualization and system
//! capabilities; the script owns composition, presentation and business logic. See
//! `docs/gpui-shell.md` for the design this implements.

// # The surface a host may rely on
//
// Everything `pub` here is a promise. That is the point of the list being
// short: a module published because one item across a crate boundary needed it
// is not thereby an interface, and the crate spent a while with `engine`,
// `spec`, `materialize` and `scope` open for exactly that reason — an external
// test needed them. Tests moved inside instead (`src/tests`), because an
// integration test is a consumer like any other and a reason to publish an
// internal representation is still a reason to publish it.
//
// **What a host may name.** The root: `init`, the `set_*` entry points,
// `on_exit_request`, `resolve_app_root`, `failure_surface`, and the types
// re-exported below. The modules: `native` and `policy` to configure what a
// script may reach, `root` and `theme` for the window it lives in, `view` and
// `snapshot` for the view itself, `watch` for hot-reload, `typings` to generate
// `gpui.d.ts`, `metrics` to measure.
//
// **Crate-private, and why.** `engine` is the seam and its shape follows
// whatever is behind it. `spec`, `materialize`, `store` and `style` are an
// internal representation. `capability` publishes `Capabilities` and
// `ExecuteGrant` through the root and keeps the resolver — `Access`, `Grant` —
// to itself. `scope` publishes `with_current_app`, which is how a native module
// reaches the ambient `App`, and hides the frame stack. `runtime`, `error` and
// `assets` publish their types through the root.
//
// **Designed, tested, not driven.** `dock`, `plugin` and `plugin_api`. Nothing
// in the crate reaches them, because a script cannot yet contribute a panel and
// no host loads a plugin. They stay private until something does; a public API
// no caller has ever exercised is a promise made on a guess.
//
// **Not reachable at all.** `value` and `entities`: a `Bridged` and an entity
// handle are the runtime talking to itself.
pub(crate) mod assets;
pub(crate) mod capability;
pub(crate) mod dock;
pub(crate) mod engine;
pub(crate) mod entities;
pub(crate) mod error;
pub(crate) mod materialize;
pub mod metrics;
pub mod native;
pub(crate) mod plugin;
pub(crate) mod plugin_api;
pub mod policy;
pub mod root;
pub(crate) mod runtime;
pub(crate) mod scope;
pub mod snapshot;
pub(crate) mod spec;
pub(crate) mod store;
pub(crate) mod style;
#[cfg(test)]
mod tests;
pub mod theme;
pub mod typings;
pub(crate) mod value;
pub mod view;
pub mod watch;

pub use assets::AppAssets;
pub use capability::{Capabilities, ExecuteGrant};
pub use engine::ShellRuntime;
pub use error::ShellError;
pub use metrics::RuntimeMetrics;
pub use native::{
    NativeArguments, NativeError, NativeModule, NativeModules, NativeObject, NativeResult,
    NativeValue,
};
pub use root::{DialogOptions, SheetSide, ShellRoot, ToastLevel, ToastRequest};
pub use runtime::{
    ExitHandler, ExitRequest, clear_exit_handler, failure_surface, on_exit_request,
    resolve_app_root,
};
pub use scope::{ScopePhase, with_current_app};
pub use snapshot::RenderSnapshot;
pub use view::ScriptView;

use std::path::PathBuf;

use gpui::App;

/// Grants an application its capabilities.
///
/// Nothing is permitted until this is called: a script gets no file, storage,
/// clipboard or process access by default (design doc §5.7). The host decides,
/// because only the host knows how much the code it is about to run is trusted.
///
/// The grant lives above the engine seam, so no engine can be built that
/// quietly ignores it. It sets the *default* policy — what a call inherits when
/// nothing narrower is in force. A host running several applications at once
/// gives each its own [`policy::Policy`] instead, so that two of them can hold
/// two grants at the same time.
pub fn set_capabilities(capabilities: Capabilities) {
    capability::install(capabilities);
}

/// Points `gpui.store` at a directory.
///
/// Storage is per application, and the host chooses where that is — an
/// application cannot name its own storage location, or two applications could
/// collide on purpose. Like [`set_capabilities`], this configures the default
/// policy.
pub fn set_store_path(path: PathBuf) {
    engine::set_store_path(path);
}

/// Relaxes the sandbox for a development session.
///
/// Restores `eval` and unfreezes the built-in prototypes, which a REPL needs
/// and a shipped application must not have.
pub fn set_development_mode(enabled: bool) {
    engine::set_development_mode(enabled);
}

/// Initializes the base layer, the shell's default semantic tokens, and the
/// style reflection table.
///
/// Must be called once at application startup, before any script runs.
pub fn init(cx: &mut App) {
    gpui_base::init(cx);
    theme::init(cx);
    style::init();
}

/// Registers the native modules a script may reach.
///
/// Nothing is reachable until this is called: `native("...")` fails while the
/// registry is empty, and it only ever resolves the modules the host put in it
/// (design doc §17.6). Call it before the application runs; the registry is
/// read at call time, so a later change takes effect on the next call.
pub fn set_native_modules(modules: NativeModules) {
    native::set_modules(modules);
}

/// Removes every native module.
///
/// A host that registered modules capturing GPUI entities should call this when
/// it goes away; see [`native::clear_modules`] for why leaving them installed is
/// a leak rather than merely untidy.
pub fn clear_native_modules() {
    native::clear_modules();
}
