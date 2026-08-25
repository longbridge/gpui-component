//! GPUI Shell — a scriptable application runtime built on `gpui-base`.
//!
//! The host owns rendering, layout, text editing, virtualization and system
//! capabilities; the script owns composition, presentation and business logic. See
//! `docs/gpui-shell.md` for the design this implements.

// # The surface a host may rely on
//
// Everything below is reachable, and not everything below is a promise. A
// module that is `pub` because something across a crate boundary needs one item
// from it is not thereby a stable interface, and saying so here is cheaper than
// discovering it in a changelog.
//
// **Host surface.** `init`, the `set_*` entry points and `on_exit_request` at
// the root; `capability`, `native`, `plugin`, `plugin_api`, `dock`, `root`,
// `theme`, `view`, `watch`, `typings`, `metrics`, `snapshot`, `error`. These
// exist for an embedder and change with notice.
//
// **Reachable, not promised.** `engine` is the seam and its shape follows
// whatever is behind it. `scope` is here because a native module needs the
// ambient `App` and there is no other way to reach it. `assets` and `style`
// are here for the binary in this package, which is a separate crate. `spec`
// and `materialize` are here so a test outside the crate can measure and assert
// on a description without a GPU — they describe an internal representation and
// will move when it does.
//
// **Not reachable at all.** `value` and `entities`: a `Bridged` and an entity
// handle are the runtime talking to itself.
pub mod assets;
pub mod capability;
pub mod dock;
pub mod engine;
pub(crate) mod entities;
pub mod error;
pub mod materialize;
pub mod metrics;
pub mod native;
pub mod plugin;
pub mod plugin_api;
pub mod root;
pub mod runtime;
pub mod scope;
pub mod snapshot;
pub mod spec;
pub mod style;
pub mod theme;
pub mod typings;
pub(crate) mod value;
pub mod view;
pub mod watch;

pub use capability::Capabilities;
pub use engine::ShellRuntime;
pub use error::ShellError;
pub use metrics::RuntimeMetrics;
pub use native::{
    NativeArguments, NativeError, NativeModule, NativeModules, NativeObject, NativeResult,
    NativeValue,
};
pub use plugin::{Plugin, PluginManager, PluginManifest};
pub use root::{DialogOptions, SheetSide, ShellRoot, ToastLevel, ToastRequest};
pub use runtime::{ExitHandler, clear_exit_handler, on_exit_request};
pub use scope::ScopePhase;
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
/// quietly ignores it.
pub fn set_capabilities(capabilities: Capabilities) {
    capability::install(capabilities);
}

/// Points `gpui.store` at a directory.
///
/// Storage is per application, and the host chooses where that is — an
/// application cannot name its own storage location, or two applications could
/// collide on purpose.
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
