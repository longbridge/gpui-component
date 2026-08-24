//! The scripting engine seam.
//!
//! Everything above this module — the spec arena, the materializer, the call
//! scope, the style table, the theme, the capability model — is engine
//! independent. Only this module knows what a script value is.
//!
//! # The contract
//!
//! An engine module must expose one type, `ShellRuntime`, with exactly this
//! surface. The rest of the crate calls nothing else, which is what makes the
//! engine replaceable:
//!
//! ```text
//! ShellRuntime::new() -> anyhow::Result<Rc<Self>>
//! ShellRuntime::set_global(&Rc<Self>, &mut App)
//! ShellRuntime::global(&App) -> Option<Rc<Self>>
//! ShellRuntime::arena_mut(&self) -> RefMut<'_, SpecArena>
//!
//! ShellRuntime::load_app(&Rc<Self>, &Path) -> anyhow::Result<ViewType>
//! ShellRuntime::load_source(&Rc<Self>, &str, &str) -> anyhow::Result<ViewType>
//! ShellRuntime::instantiate(&Rc<Self>, &ViewType) -> anyhow::Result<ViewObject>
//!
//! ShellRuntime::render_view(&Rc<Self>, ViewObject, Entity<ScriptView>, &mut Window, &mut App)
//!     -> AnyElement
//! ShellRuntime::render_to_spec(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
//!     &mut Window, &mut App) -> anyhow::Result<String>
//!
//! ShellRuntime::dispatch_click(&Rc<Self>, CallbackId, &ClickEvent, &mut Window, &mut App)
//! ShellRuntime::dispatch_change(&Rc<Self>, CallbackId, bool, &mut Window, &mut App)
//! ```
//!
//! plus the associated handle types `ViewType` and `ViewObject`, which are
//! opaque to every caller.
//!
//! # Why the seam exists
//!
//! The engine choice is the one decision in this runtime that cannot be
//! validated on paper: per-call cost across the language boundary decides
//! whether the whole approach is viable (see `docs/research/gpui-shell.md`
//! §20). QuickJS is the default because application code reads better in
//! JavaScript; the Lua engine stays behind a feature flag so the measurement
//! can be run against both, and so switching back is a feature change rather
//! than a rewrite.

#[cfg(all(feature = "quickjs", any(feature = "lua", feature = "luajit")))]
compile_error!(
    "enable exactly one scripting engine: `quickjs` (default) or `lua`/`luajit`. \
     Building both would make `gpui_shell::ShellRuntime` ambiguous; disable default \
     features to select another engine."
);

#[cfg(not(any(feature = "quickjs", feature = "lua", feature = "luajit")))]
compile_error!("enable one scripting engine: `quickjs` (default) or `lua`/`luajit`");

#[cfg(feature = "quickjs")]
pub(crate) mod quickjs;
#[cfg(feature = "quickjs")]
pub use quickjs::{ShellRuntime, ViewObject, ViewType};

#[cfg(any(feature = "lua", feature = "luajit"))]
mod lua;
#[cfg(any(feature = "lua", feature = "luajit"))]
pub use lua::{ShellRuntime, ViewObject, ViewType};
