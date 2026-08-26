//! GPUI Shell — a scriptable application runtime built on `gpui-base`.
//!
//! > **Experimental:** scripting interfaces, Standard Runtime compatibility,
//! > capability semantics and module behavior may change between minor
//! > releases.
//!
//! The host owns rendering, layout, text editing, virtualization and system
//! capabilities; the script owns composition, presentation and business logic. See
//! `docs/gpui-shell.md` for the design this implements.
//!
//! # Cargo feature impact
//!
//! The JavaScript fluent-style surface is generated from GPUI's inspector
//! reflection table, including in release builds. This crate therefore enables
//! `gpui-base/inspector`, which Cargo unifies across the embedding application's
//! dependency graph.

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
// `snapshot` for the view itself, `watch` for hot-reload, `metrics` to measure.
// `write_type_declarations` is the explicit tooling hook for `gpui.d.ts`;
// ordinary application loading updates it automatically.
//
// **Crate-private, and why.** `engine` is the seam and its shape follows
// whatever is behind it. `spec`, `materialize`, `store`, `style` and `a11y` are
// an internal representation. `capability` publishes `Capabilities` and
// `ExecuteGrant` through the root and keeps the resolver — `Access`, `Grant` —
// to itself. `scope` publishes `with_current_app`, which is how a native module
// reaches the ambient `App`, and hides the frame stack. `scroll` is the one
// scroll area `materialize` needs, kept here because the shell builds on
// `gpui-base` alone and cannot borrow `gpui-component`'s copy. `runtime`,
// `error` and `assets` publish their types through the root.
//
// **Designed, tested, not driven.** `dock`. A script cannot yet
// contribute a panel. The plugin manifest is public because the shipped binary
// now applies a local application's declared capabilities before loading it.
//
// **Not reachable at all.** `value` and `entities`: a `Bridged` and an entity
// handle are the runtime talking to itself.
pub(crate) mod a11y;
pub(crate) mod assets;
pub(crate) mod capability;
pub(crate) mod dock;
pub(crate) mod engine;
pub(crate) mod entities;
pub(crate) mod error;
pub(crate) mod materialize;
pub(crate) mod path;
pub mod metrics;
pub mod native;
pub mod plugin;
pub mod policy;
pub(crate) mod process;
pub mod root;
pub(crate) mod runtime;
pub(crate) mod scope;
pub(crate) mod scroll;
pub mod snapshot;
pub(crate) mod spec;
pub(crate) mod store;
pub(crate) mod style;
#[cfg(test)]
mod tests;
pub mod theme;
mod typings;
pub(crate) mod value;
pub mod view;
pub mod watch;

pub use assets::AppAssets;
pub use capability::{Capabilities, ExecuteGrant, HttpRequestGrant};
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

/// Writes current `gpui.d.ts` declarations into an application tree.
///
/// [`ShellRuntime::load`] already performs this best-effort for ordinary hosts.
/// This explicit operation exists for tooling such as `gpui-shell types` that
/// must report a write failure to its caller.
pub fn write_type_declarations(root: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    typings::write_application(root)
}

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

/// Names the application, and puts its data where that name says.
///
/// **This is how a host should place storage.** The bundle id is the
/// application's identity, so its data survives the directory being renamed,
/// moved, or replaced by an upgrade — which is what a user means by "my
/// settings". Keying on the path instead means an upgrade silently starts the
/// user over.
///
/// The id is the host's to decide and the runtime does not go looking for it in
/// a file: only the layer that installed the application knows what it is
/// called, and a runtime that read it out of a manifest of its own choosing
/// would be claiming authority over something it does not own.
///
/// Returns the directory it chose, because a host that grants filesystem access
/// needs to name it. The store is one file inside, which leaves room for other
/// per-application state later.
///
/// ```rust,ignore
/// let data = gpui_shell::set_bundle_id("com.example.notes")?;
/// gpui_shell::set_capabilities(Capabilities::new().write_roots([data]));
/// ```
///
/// A host running a directory it was pointed at — a command line, a dev
/// server — has no such name, and passing the path is right there: the path is
/// the identity while you are editing something. [`bundle_id_for_path`] builds
/// one.
///
/// Fails when the id could reach outside the data directory: it is joined onto
/// it, so `a-z`, `0-9`, `.`, `-`, `_` and no `..`.
pub fn set_bundle_id(id: &str) -> anyhow::Result<PathBuf> {
    let directory = runtime::app_data_dir(id)?;
    set_store_path(directory.join("store.json"));
    Ok(directory)
}

/// A bundle id for a directory that has no name of its own.
///
/// The directory name with a digest of its full path: the same directory always
/// reaches the same data, and two never collide — including two checkouts of one
/// source, which really are two installations of something being edited.
pub fn bundle_id_for_path(root: &std::path::Path) -> String {
    runtime::path_identity(root)
}

/// Points `gpui.store` at an exact file.
///
/// The mechanism under [`set_bundle_id`], which is what a host should normally
/// call. This is for a host that places its own data — a test, or an embedder
/// with its own layout.
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
/// it goes away; leaving them installed is
/// a leak rather than merely untidy.
pub fn clear_native_modules() {
    native::clear_modules();
}
