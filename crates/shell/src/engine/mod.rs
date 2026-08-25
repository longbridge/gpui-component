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
//! ShellRuntime::load_app(&Rc<Self>, &Path, entry: &str) -> anyhow::Result<ViewType>
//! ShellRuntime::load_source(&Rc<Self>, &str, &str) -> anyhow::Result<ViewType>
//! ShellRuntime::instantiate(&Rc<Self>, &ViewType) -> anyhow::Result<ViewObject>
//!
//! ShellRuntime::build_snapshot(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
//!     &mut Window, &mut App) -> anyhow::Result<RenderSnapshot>
//! ShellRuntime::render_to_spec(&Rc<Self>, &ViewObject, Option<Entity<ScriptView>>,
//!     &mut Window, &mut App) -> anyhow::Result<String>
//! ShellRuntime::retire_callbacks(&self, generation: u32)
//! ShellRuntime::script_renders(&self) -> u64
//!
//! ShellRuntime::dispatch_click(&Rc<Self>, CallbackId, &ClickEvent, &mut Window, &mut App)
//! ShellRuntime::dispatch_change(&Rc<Self>, CallbackId, bool, &mut Window, &mut App)
//! ```
//!
//! plus the associated handle types `ViewType` and `ViewObject`, which are
//! opaque to every caller.
//!
//! `arena_mut` is the *scratch* arena the script builder records into during a
//! `build_snapshot` call. It is reset at the start of every build and taken at
//! the end; nothing outside a build should read it. Published descriptions live
//! in [`crate::snapshot::RenderSnapshot`].
//!
//! # The one invariant an engine must not break
//!
//! `build_snapshot` is the only entry into the script's `render`, and nothing
//! calls it per frame. An engine that renders opportunistically — on a repaint,
//! on a hover, on a timer — would put script cost back on GPUI's frame budget,
//! which is the coupling this seam exists to prevent. `script_renders` is the
//! counter that lets a test prove it did not happen.
//!
//! # Why the seam exists
//!
//! The engine choice is the one decision in this runtime that cannot be
//! validated on paper: per-call cost across the language boundary decides
//! whether the whole approach is viable (see `docs/gpui-shell.md` §20). QuickJS
//! is what ships, because application code reads better in JavaScript.
//!
//! The seam is not speculative generality. Everything above it — the snapshot,
//! the spec arena, the materializer, the call scope, the style table, the theme,
//! the capability model — is written against the contract rather than against
//! QuickJS, which is what would make a second engine a new module instead of a
//! rewrite. Nothing outside this module names a script value.

#[cfg(not(feature = "quickjs"))]
compile_error!("enable a scripting engine: `quickjs` is the default and the only one today");

#[cfg(feature = "quickjs")]
pub(crate) mod quickjs;
#[cfg(feature = "quickjs")]
pub use quickjs::{ShellRuntime, ViewObject, ViewType};
