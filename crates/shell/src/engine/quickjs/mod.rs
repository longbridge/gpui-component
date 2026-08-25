//! The QuickJS engine.
//!
//! Application code is JavaScript: ES modules, classes, arrow functions. This
//! module is the only place that knows that — everything above `engine/` deals
//! in [`SpecId`]s, [`Bridged`] values and [`ShellError`]s.
//!
//! Two shapes are worth knowing before reading:
//!
//! - **Elements are plain JS objects** carrying an integer `__id`, sharing one
//!   prototype that holds every bound method. A method call is therefore an
//!   ordinary prototype lookup rather than a proxy trap, which matters because
//!   per-call cost is the whole viability question (design doc §20).
//! - **The prototype is built by a JS prelude**, not by 3000 Rust closures: the
//!   prelude loops over the style-name list and installs one small JS function
//!   per name, each forwarding to a single Rust entry point.

use std::{
    cell::{Cell, RefCell, RefMut},
    path::Path,
    rc::{Rc, Weak},
};

use anyhow::{Context as _, Result, anyhow};
use gpui::{App, ClickEvent, Entity, Global, Window};
use rquickjs::{
    Context as JsContext, Ctx, Error as JsError, Exception, FromJs, Function, Object, Persistent,
    Result as JsResult, Runtime as JsRuntime, Value,
    function::{Func, This},
    loader::{BuiltinResolver, ImportAttributes, Loader, ModuleLoader, Resolver},
    module::Declared,
    module::{Declarations, Exports, Module, ModuleDef},
};
use smallvec::SmallVec;

use crate::{
    entities::EntityStore,
    metrics::Metrics,
    runtime::{CallbackArena, CallbackEntry},
    scope::{self, ScopePhase},
    snapshot::RenderSnapshot,
    spec::{CallbackId, Component, SpecArena, SpecId, SpecOp},
    style,
    value::Bridged,
    view::ScriptView,
};

/// A script value that defines a view type — a JS class.
pub type ViewType = Persistent<Object<'static>>;
/// One instance of a view type.
pub type ViewObject = Persistent<Object<'static>>;

mod entity_api;
pub(crate) mod host;
mod native;
mod overlay;
pub(crate) mod sandbox;
mod scheduler;
mod theme_api;

/// Names exported by the built-in `gpui` module.
///
/// Anything installed onto `globalThis.__gpui` must be listed here or
/// `import { … } from "gpui"` will not see it.
const MODULE_EXPORTS: &[&str] = &[
    // Elements and views.
    "View",
    "div",
    "h_flex",
    "v_flex",
    "text",
    "svg",
    "Button",
    "Checkbox",
    "Switch",
    "Input",
    "InputState",
    // System capabilities (`host`, `sandbox`).
    "fs",
    "process",
    "store",
    "clipboard",
    "log",
    // Native modules (`native`).
    "native",
    // Theme and runtime version (`theme_api`).
    "theme",
    "set_theme",
    "require_api",
    // Scheduling (`scheduler`).
    "spawn",
    "timer",
    "sleep",
    "with_cx",
];

pub struct ShellRuntime {
    /// Declared first because fields drop in declaration order and every
    /// `Persistent` handle must be released while the context still exists.
    /// QuickJS aborts the process if a value outlives its runtime.
    callbacks: RefCell<CallbackArena<Persistent<Function<'static>>>>,
    arena: RefCell<SpecArena>,
    /// Retained state created by this runtime's scripts, and only this one's.
    /// Declared before `context` for the same reason `callbacks` is: releasing
    /// an entity can run script destructors.
    entities: RefCell<EntityStore>,
    /// What the runtime is spending. See [`Self::metrics`].
    metrics: Metrics,
    context: JsContext,
    /// Incremented per `load_app`, so a reload re-reads every module rather
    /// than serving the first version from QuickJS's module cache.
    module_generation: Rc<Cell<u32>>,
    /// Held so the context stays alive, and so the module loader can be scoped
    /// to an application directory when one is loaded.
    js_runtime: JsRuntime,
}

impl Drop for ShellRuntime {
    fn drop(&mut self) {
        // Both hold `Persistent` script values, and a persistent handle
        // released after its runtime aborts the process.
        scheduler::shutdown();
        self.callbacks.borrow_mut().clear();
        // Retained entities are owned by GPUI but reachable only through this
        // runtime's handles; leaving them registered outlives the app that owns
        // them, which GPUI reports as a leaked handle on shutdown.
        self.entities.borrow_mut().clear();
    }
}

struct RuntimeGlobal(Rc<ShellRuntime>);

impl Global for RuntimeGlobal {}

impl ShellRuntime {
    pub fn new() -> Result<Rc<Self>> {
        let js_runtime = JsRuntime::new().map_err(js_setup_error)?;
        let context = JsContext::full(&js_runtime).map_err(js_setup_error)?;

        js_runtime.set_loader(
            BuiltinResolver::default().with_module("gpui"),
            ModuleLoader::default().with_module("gpui", GpuiModule),
        );

        // Resource limits belong to the sandbox policy, but only the engine
        // owns the runtime handle, so the policy hands out values and this is
        // where they are applied. A runaway script must not be able to hold the
        // UI thread (§19.3).
        js_runtime.set_memory_limit(sandbox::memory_limit_bytes());
        js_runtime.set_max_stack_size(sandbox::max_stack_size_bytes());
        js_runtime.set_interrupt_handler(Some(Box::new(sandbox::interrupt_handler())));

        let runtime = Rc::new(Self {
            callbacks: RefCell::new(CallbackArena::default()),
            arena: RefCell::new(SpecArena::new()),
            entities: RefCell::new(EntityStore::new()),
            metrics: Metrics::default(),
            context,
            module_generation: Rc::new(Cell::new(0)),
            js_runtime,
        });

        runtime.install_globals()?;
        Ok(runtime)
    }

    pub fn set_global(self: &Rc<Self>, cx: &mut App) {
        cx.set_global(RuntimeGlobal(self.clone()));
    }

    pub fn global(cx: &App) -> Option<Rc<Self>> {
        cx.try_global::<RuntimeGlobal>()
            .map(|global| global.0.clone())
    }

    pub fn arena_mut(&self) -> RefMut<'_, SpecArena> {
        self.arena.borrow_mut()
    }

    /// What the runtime is spending: script renders and materializations, with
    /// the time each took.
    ///
    /// The two counters follow different things — application activity and
    /// frame count — and the gap between them is what the snapshot lifecycle
    /// exists to produce. See [`crate::metrics`].
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// This runtime's retained state.
    ///
    /// Scoped to the runtime rather than shared, so one runtime cannot resolve
    /// another's handle — see [`crate::entities`].
    pub fn entities(&self) -> RefMut<'_, EntityStore> {
        self.entities.borrow_mut()
    }

    /// Loads `main.js` from an application directory.
    ///
    /// Module resolution is scoped to that directory: an application can import
    /// its own files and the built-in `gpui` module, and nothing else. That is
    /// the first half of the sandbox's module policy (design doc §19.1).
    pub fn load_app(self: &Rc<Self>, dir: &Path, entry: &str) -> Result<ViewType> {
        let root = crate::runtime::resolve_app_root(dir, entry)?;

        // Every load is a new generation, which is what makes a reload pick up
        // a change in an imported module rather than only in the entry point.
        self.module_generation
            .set(self.module_generation.get().wrapping_add(1));

        self.js_runtime.set_loader(
            (
                BuiltinResolver::default().with_module("gpui"),
                AppModules::new(root.clone(), self.module_generation.clone()),
            ),
            (
                ModuleLoader::default().with_module("gpui", GpuiModule),
                AppModules::new(root.clone(), self.module_generation.clone()),
            ),
        );

        let entry = root.join(entry);
        let source = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        // The entry carries the generation too: it is a cached module like any
        // other, and a reload that re-read every import but served a stale
        // `main.js` would be the same bug one level up.
        self.load_source(
            &format!(
                "{}?v={}",
                entry.to_string_lossy(),
                self.module_generation.get()
            ),
            &source,
        )
    }

    /// Evaluates a module and returns its default export, which must be a view
    /// class.
    pub fn load_source(self: &Rc<Self>, name: &str, source: &str) -> Result<ViewType> {
        self.with_js(|ctx| {
            let (module, promise) = rquickjs::Module::declare(ctx.clone(), name, source)?.eval()?;
            promise.finish::<()>()?;

            let default: Value = module.get("default")?;
            let Some(class) = default.as_object() else {
                return Err(Exception::throw_message(
                    ctx,
                    "main.js must `export default` a class that extends View",
                ));
            };
            Ok(Persistent::save(ctx, class.clone()))
        })
    }

    /// Constructs one instance of a view class.
    ///
    /// `init` is where a view creates the state it keeps across frames, and
    /// creating an entity needs a `Window` and an `App`. So construction opens
    /// a scope of its own rather than running in the gap between host calls.
    pub fn instantiate(
        self: &Rc<Self>,
        view_type: &ViewType,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ViewObject> {
        let (_guard, _generation) = scope::enter(window, cx, ScopePhase::Event, None);
        self.with_js(|ctx| {
            let class = view_type.clone().restore(ctx)?;
            let construct: Function = ctx.globals().get("__construct")?;
            let instance: Object = construct.call((class,))?;
            Ok(Persistent::save(ctx, instance))
        })
    }

    /// Runs the script's `render` and freezes what it described.
    ///
    /// This is the only path into the VM's render function, and it is called
    /// only when a view says its description may be out of date — never once
    /// per frame. Everything it produces belongs to the returned snapshot:
    /// the element descriptions, the root, and the handlers registered while
    /// building it.
    ///
    /// The build is transactional. The scratch arena and an open callback
    /// generation are staging; they are published together at the end, and a
    /// script that throws discards both, leaving whatever snapshot the caller
    /// already had untouched.
    pub fn build_snapshot(
        self: &Rc<Self>,
        object: &ViewObject,
        view: Option<Entity<ScriptView>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<RenderSnapshot> {
        self.arena.borrow_mut().reset();
        let callbacks = self.callbacks.borrow_mut().begin();

        let root = self.metrics.time_script_render(|| {
            let (_guard, generation) = scope::enter(window, cx, ScopePhase::Render, view.clone());
            self.call_render(object, generation)
        });

        let root = match root {
            Ok(root) => root,
            Err(error) => {
                self.callbacks.borrow_mut().abort();
                self.arena.borrow_mut().reset();
                return Err(error);
            }
        };

        self.callbacks.borrow_mut().commit();
        // Taking the arena publishes the description and leaves a fresh scratch
        // arena behind, so the snapshot owns its nodes outright rather than
        // sharing them with the next build.
        let arena = std::mem::take(&mut *self.arena.borrow_mut());
        let snapshot = RenderSnapshot::new(self, callbacks, root, arena);

        // Promise callbacks only run when the host drains QuickJS's job queue.
        // That drain is deferred to the event loop rather than run here: a
        // continuation is application code of unbounded length, and a render is
        // the last path it belongs on. It costs one check when nothing is
        // queued, which is the usual case.
        if let Some(view) = view {
            scheduler::drain_after_render(&self.js_runtime, view, window, cx);
        }

        Ok(snapshot)
    }

    /// Runs the script and returns the element description as text.
    ///
    /// The description is plain data, so interface structure can be asserted in
    /// tests that never paint a frame. This runs the script; to read a
    /// description that has already been built, use
    /// [`RenderSnapshot::debug_tree`] instead — that path never enters the VM.
    pub fn render_to_spec(
        self: &Rc<Self>,
        object: &ViewObject,
        view: Option<Entity<ScriptView>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<String> {
        Ok(self.build_snapshot(object, view, window, cx)?.debug_tree())
    }

    /// Releases the handlers registered while one snapshot was built.
    ///
    /// Called by [`RenderSnapshot`] as it drops, which is what ties handler
    /// lifetime to snapshot lifetime rather than to a frame.
    pub fn retire_callbacks(&self, generation: u32) {
        self.callbacks.borrow_mut().retire(generation);
    }

    pub fn dispatch_click(
        self: &Rc<Self>,
        id: CallbackId,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("click callback {id} belongs to a superseded render pass");
            return;
        };

        let (_guard, generation) = scope::enter(window, cx, ScopePhase::Event, entry.view.clone());
        let click_count = event.click_count();
        let modifiers = event.modifiers();

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            payload.set("click_count", click_count)?;

            let flags = Object::new(ctx.clone())?;
            flags.set("shift", modifiers.shift)?;
            flags.set("control", modifiers.control)?;
            flags.set("alt", modifiers.alt)?;
            flags.set("platform", modifiers.platform)?;
            payload.set("modifiers", flags)?;

            handler.call::<_, ()>((payload, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in click handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Delivers an input event to a long-lived script subscription.
    ///
    /// Unlike a rendered callback this handler outlives the pass that created
    /// it, so it lives with the entity rather than in the per-frame arena.
    pub(super) fn dispatch_input_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        event: &gpui_base::input::InputEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_base::input::InputEvent;

        let (_guard, generation) = scope::enter(window, cx, ScopePhase::Event, None);

        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            match event {
                InputEvent::PressEnter { secondary, shift } => {
                    payload.set("secondary", *secondary)?;
                    payload.set("shift", *shift)?;
                }
                InputEvent::Change | InputEvent::Focus | InputEvent::Blur => {}
            }
            handler.call::<_, ()>((payload, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in input handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Controlled-value handlers report intent; the script stores the value and
    /// notifies. The host never mutates script state on its behalf.
    pub fn dispatch_change(
        self: &Rc<Self>,
        id: CallbackId,
        checked: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("change callback {id} belongs to a superseded render pass");
            return;
        };

        let (_guard, generation) = scope::enter(window, cx, ScopePhase::Event, entry.view.clone());

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((checked, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in change handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Renders once, and on a "not a function" failure renders again with the
    /// diagnostic prototype installed so the error can name the method and
    /// suggest a correction. See the prelude for why this is two passes.
    fn call_render(&self, object: &ViewObject, generation: u64) -> Result<SpecId> {
        match self.call_render_once(object, generation) {
            Ok(id) => Ok(id),
            Err(error) if error.to_string().contains("not a function") => {
                self.set_diagnostics(true);
                self.arena.borrow_mut().reset();
                // The first attempt already recorded handlers into the open
                // generation; the retry describes the same tree again, so it
                // must start from an empty index space.
                self.callbacks.borrow_mut().rollback();
                let diagnosed = self.call_render_once(object, generation);
                self.set_diagnostics(false);
                match diagnosed {
                    Ok(id) => Ok(id),
                    Err(better) => Err(better),
                }
            }
            Err(error) => Err(error),
        }
    }

    fn set_diagnostics(&self, enabled: bool) {
        let _ = self.with_js(|ctx| ctx.globals().set("__diagnostics", enabled));
    }

    fn call_render_once(&self, object: &ViewObject, generation: u64) -> Result<SpecId> {
        self.with_js(|ctx| {
            let instance = object.clone().restore(ctx)?;
            let render: Function = instance.get("render").map_err(|_| {
                Exception::throw_message(ctx, "view class has no render(cx) method")
            })?;
            let produced: Value =
                render.call((This(instance), context_object(ctx, generation)?))?;
            element_id(ctx, &produced)
        })
    }

    /// Runs `body` inside the JS context, flattening any exception into an
    /// ordinary error carrying the script's message and stack.
    fn with_js<T>(&self, body: impl FnOnce(&Ctx<'_>) -> JsResult<T>) -> Result<T> {
        self.context.with(|ctx| match body(&ctx) {
            Ok(value) => Ok(value),
            Err(error) => Err(anyhow!("{}", describe(&ctx, error))),
        })
    }

    /// Opens a detached node that collects the declarations of one state style.
    fn begin_state(&self, ctx: &Ctx<'_>, id: SpecId, name: &str) -> JsResult<SpecId> {
        let interned = match name {
            "hover" => "hover",
            "active" => "active",
            "focus" => "focus",
            other => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!("unknown state style `{other}`; expected hover, active or focus"),
                ));
            }
        };

        let node = self.arena.borrow_mut().push(Component::Div);
        self.arena
            .borrow_mut()
            .claim(node)
            .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
        self.push_op_checked(ctx, self.push_op(id, SpecOp::StateStyle(interned, node)))?;
        Ok(node)
    }

    fn push_node(&self, component: Component) -> SpecId {
        self.arena.borrow_mut().push(component)
    }

    /// Records an element for a component the bindings build themselves.
    pub(super) fn push_component(&self, component: Component) -> SpecId {
        self.push_node(component)
    }

    fn push_op(&self, id: SpecId, op: SpecOp) -> Result<(), crate::spec::SpecError> {
        self.arena.borrow_mut().push_op(id, op)
    }
}

/// Resolves and loads an application's own modules, and nothing else.
///
/// `FileResolver` from rquickjs is not usable here: it tests candidate paths
/// relative to the process working directory, so an absolute application path
/// never matches. Owning the resolver also puts the sandbox's module policy in
/// one place — a module must live inside the application root, which is what
/// stops `import "../../../etc/passwd"` before it reaches the filesystem.
#[derive(Clone)]
struct AppModules {
    root: std::path::PathBuf,
    /// Bumped on every load so a reload re-reads every file.
    ///
    /// QuickJS caches an evaluated module by name, and an ES module cannot be
    /// unloaded — so re-evaluating `main.js` alone left every module it imports
    /// at the version that was on disk the first time. A hot reload that
    /// silently ignores every file except the entry point is worse than no hot
    /// reload, because it looks like it worked. Tagging the resolved name with
    /// a generation makes each reload a different module as far as the cache is
    /// concerned. The previous generation stays in the cache until the runtime
    /// shuts down; that is the cost, and it is a development-only one.
    generation: Rc<Cell<u32>>,
}

impl AppModules {
    fn new(root: std::path::PathBuf, generation: Rc<Cell<u32>>) -> Self {
        Self { root, generation }
    }

    /// Strips the generation tag a resolved name carries.
    fn untag(name: &str) -> &str {
        name.split_once("?v=").map(|(path, _)| path).unwrap_or(name)
    }

    fn candidate(&self, base: &str, name: &str) -> Option<std::path::PathBuf> {
        let start = if name.starts_with('.') {
            Path::new(Self::untag(base)).parent()?.to_path_buf()
        } else {
            self.root.clone()
        };

        let joined = start.join(name);
        for candidate in [joined.clone(), joined.with_extension("js")] {
            if candidate.is_file() {
                return candidate.canonicalize().ok();
            }
        }
        None
    }
}

impl Resolver for AppModules {
    fn resolve<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<String> {
        let Some(path) = self.candidate(base, name) else {
            return Err(Exception::throw_message(
                ctx,
                &format!("cannot resolve module `{name}` from `{base}`"),
            ));
        };

        if !path.starts_with(&self.root) {
            return Err(Exception::throw_message(
                ctx,
                &format!(
                    "module `{name}` resolves outside the application directory `{}`",
                    self.root.display()
                ),
            ));
        }

        Ok(format!(
            "{}?v={}",
            path.to_string_lossy(),
            self.generation.get()
        ))
    }
}

impl Loader for AppModules {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> JsResult<Module<'js, Declared>> {
        let path = Self::untag(name);
        let source = std::fs::read_to_string(path).map_err(|error| {
            Exception::throw_message(ctx, &format!("cannot read module `{path}`: {error}"))
        })?;
        Module::declare(ctx.clone(), name, source)
    }
}

/// The built-in `gpui` module. Its values are built at startup and stashed on
/// the global object; this only re-exports them under module names so that
/// `import { div } from "gpui"` works.
struct GpuiModule;

impl ModuleDef for GpuiModule {
    fn declare(declarations: &Declarations) -> JsResult<()> {
        for name in MODULE_EXPORTS {
            declarations.declare(*name)?;
        }
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> JsResult<()> {
        let module: Object = ctx.globals().get("__gpui")?;
        for name in MODULE_EXPORTS {
            let value: Value = module.get(*name)?;
            exports.export(*name, value)?;
        }
        Ok(())
    }
}

/// Installed once per context. It builds the element prototype from the style
/// name list, which is why adding a style method upstream needs no change here.
const PRELUDE: &str = r#"
globalThis.__gpui = (() => {
  const methods = {};

  // Two prototypes, and a measured reason for having two.
  //
  // QuickJS reports a missing method as `TypeError: not a function` without
  // naming the property, so a mistyped style name would arrive with no clue —
  // and giving the call site a real diagnostic is the entire reason the style
  // surface is methods rather than a string of class names (§13.2).
  //
  // A Proxy prototype solves that, but the M0 benchmark measured it at ~30% of
  // the whole description pass (1.09 ms → 1.42 ms for 443 nodes). So the fast
  // prototype is the default, and a render that fails with "not a function" is
  // re-run once against the diagnostic prototype purely to produce the message.
  // Errors are rare; a 30% tax on every frame is not.
  const diagnostic = new Proxy(methods, {
    get(target, name, receiver) {
      const found = Reflect.get(target, name, receiver);
      if (found !== undefined) return found;
      if (typeof name !== "string" || name.startsWith("__")) return undefined;
      return () => __unknown(name);
    },
  });

  globalThis.__diagnostics = false;

  const element = (id) => {
    const object = Object.create(globalThis.__diagnostics ? diagnostic : methods);
    object.__id = id;
    return object;
  };

  const define = (name) => {
    methods[name] = function (...args) {
      __apply(this.__id, name, args);
      return this;
    };
  };

  for (const name of __styleNames) define(name);
  for (const name of __behaviorNames) define(name);

  methods.child = function (child) {
    __apply(this.__id, "child", [child.__id]);
    return this;
  };
  methods.children = function (list) {
    for (const child of list) __apply(this.__id, "child", [child.__id]);
    return this;
  };
  // State styles reuse the ordinary style methods on a detached element, so
  // there is no second grammar for "what a style is".
  const state = (name) =>
    function (declare) {
      const target = element(__state(this.__id, name));
      declare(target);
      return this;
    };

  methods.hover = state("hover");
  methods.active = state("active");
  methods.focus = state("focus");

  methods.when = function (condition, branch) {
    if (!condition) return this;
    const produced = branch(this);
    if (produced === undefined || produced === null) {
      throw new Error("when(...) must return the element");
    }
    return produced;
  };

  // Retained state is held by handle; the methods close over it so nothing has
  // to read it back off `this`.
  const inputState = (handle) => ({
    __handle: handle,
    value: () => __input_value(handle),
    set_value: (next) => __input_set_value(handle, String(next ?? "")),
    on: (event, handler) => __input_on(handle, String(event), handler),
    release: () => __input_release(handle),
  });

  // `console` is the first thing a JavaScript author types. It is an alias for
  // `gpui.log`, not a second logging system: same sink, same plugin field.
  // Resolved at call time, not here: `gpui.log` is installed after this
  // prelude runs.
  const forward = (level) => (...args) => globalThis.__gpui.log[level](...args);
  globalThis.console = {
    debug: forward("debug"),
    log: forward("info"),
    info: forward("info"),
    warn: forward("warn"),
    error: forward("error"),
  };

  globalThis.__construct = (Class) => new Class();

  class View {
    constructor(props) {
      if (typeof this.init === "function") this.init(props);
    }
  }

  return {
    View,
    div: () => element(__div()),
    h_flex: () => element(__h_flex()),
    v_flex: () => element(__v_flex()),
    text: (value) => element(__text(String(value))),
    svg: (path) => element(__svg(String(path))),
    theme: () => JSON.parse(__theme_snapshot()),
    Button: { new: (id) => element(__button(String(id))) },
    Checkbox: { new: (id) => element(__checkbox(String(id))) },
    Switch: { new: (id) => element(__switch(String(id))) },
    InputState: {
      new: (options) =>
        inputState(__input_state_new(options?.placeholder ?? null, options?.value ?? null)),
    },
    Input: { new: (state) => element(__input_element(state.__handle)) },
  };
})();
"#;

impl ShellRuntime {
    fn install_globals(self: &Rc<Self>) -> Result<()> {
        let runtime = Rc::downgrade(self);
        self.with_js(move |ctx| {
            let globals = ctx.globals();

            let names = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in style::known_names().into_iter().enumerate() {
                names.set(index, name)?;
            }
            globals.set("__styleNames", names)?;

            let behaviors = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in [
                "on_click",
                "on_change",
                "disabled",
                "selected",
                "checked",
                "accessibility_label",
                "id",
            ]
            .into_iter()
            .enumerate()
            {
                behaviors.set(index, name)?;
            }
            globals.set("__behaviorNames", behaviors)?;

            constructor(&globals, "__div", runtime.clone(), || Component::Div)?;
            constructor(&globals, "__h_flex", runtime.clone(), || Component::HFlex)?;
            constructor(&globals, "__v_flex", runtime.clone(), || Component::VFlex)?;
            text_constructor(&globals, "__text", runtime.clone(), Component::Text)?;
            text_constructor(&globals, "__svg", runtime.clone(), Component::Svg)?;
            text_constructor(&globals, "__button", runtime.clone(), Component::Button)?;
            text_constructor(&globals, "__checkbox", runtime.clone(), Component::Checkbox)?;
            text_constructor(&globals, "__switch", runtime.clone(), Component::Switch)?;

            let state_runtime = runtime.clone();
            globals.set(
                "__state",
                Func::from(
                    move |ctx: Ctx<'_>, id: u32, name: String| -> JsResult<SpecId> {
                        upgrade(&state_runtime, &ctx)?.begin_state(&ctx, id, &name)
                    },
                ),
            )?;

            globals.set(
                "__unknown",
                Func::from(|ctx: Ctx<'_>, name: String| -> JsResult<()> {
                    Err(Exception::throw_type(&ctx, &unknown_method(&name)))
                }),
            )?;

            let apply_runtime = runtime.clone();
            globals.set(
                "__apply",
                Func::from(
                    move |ctx: Ctx<'_>, id: u32, name: String, args: Arguments| {
                        let runtime = upgrade(&apply_runtime, &ctx)?;
                        runtime.apply(&ctx, id, &name, args)
                    },
                ),
            )?;

            ctx.eval::<(), _>(PRELUDE)?;

            // Subsystems extend the same module object the prelude built.
            let module: Object = ctx.globals().get("__gpui")?;
            host::install(ctx, &module)?;
            native::install(ctx, &module)?;
            theme_api::install(ctx, &module)?;
            entity_api::install(ctx, &module, runtime.clone())?;
            scheduler::install(ctx, &module)?;
            sandbox::install(ctx)?;

            Ok(())
        })
    }

    fn apply(&self, ctx: &Ctx<'_>, id: SpecId, method: &str, args: Arguments) -> JsResult<()> {
        match method {
            "child" => {
                let child = args
                    .first_value()
                    .and_then(|value| value.as_f32().ok())
                    .ok_or_else(|| {
                        Exception::throw_type(ctx, "child(element) expects an element")
                    })? as SpecId;
                let attached = self.arena.borrow_mut().attach(id, child);
                self.push_op_checked(ctx, attached)
            }
            "on_click" | "on_change" => {
                let saved = args.first_handler().ok_or_else(|| {
                    Exception::throw_type(ctx, &format!("{method}(handler) expects a function"))
                })?;
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: saved,
                    view: scope::current_view(),
                });
                let name = if method == "on_click" {
                    "on_click"
                } else {
                    "on_change"
                };
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Callback(name, callback)))
            }
            "disabled" | "selected" | "checked" | "accessibility_label" | "id" => {
                let bridged = args.values(method)?;
                let name = match method {
                    "disabled" => "disabled",
                    "selected" => "selected",
                    "checked" => "checked",
                    "id" => "id",
                    _ => "accessibility_label",
                };
                if name == "id"
                    && bridged
                        .first()
                        .and_then(|value| value.as_str().ok())
                        .is_none()
                {
                    return Err(Exception::throw_type(
                        ctx,
                        "id(name) expects a string; it is the element's stable identity, so it \
                         must not change between renders",
                    ));
                }
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Method(name, bridged)))
            }
            _ => {
                if let Some(index) = style::nullary_index(method) {
                    return self
                        .push_op_checked(ctx, self.push_op(id, SpecOp::NullaryStyle(index)));
                }
                if let Some(name) = style::param_style_name(method) {
                    let bridged = args.values(name)?;
                    // Validate eagerly so a bad argument reports at the call
                    // site instead of surfacing during materialize.
                    style::apply_param(name, &bridged, Default::default())
                        .map_err(|error| Exception::throw_type(ctx, error.message()))?;
                    return self
                        .push_op_checked(ctx, self.push_op(id, SpecOp::ParamStyle(name, bridged)));
                }
                Err(Exception::throw_type(ctx, &unknown_method(method)))
            }
        }
    }

    fn push_op_checked<E: std::fmt::Display>(
        &self,
        ctx: &Ctx<'_>,
        result: Result<(), E>,
    ) -> JsResult<()> {
        result.map_err(|error| Exception::throw_type(ctx, &error.to_string()))
    }
}

fn constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    build: fn() -> Component,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(move |ctx: Ctx<'_>| -> JsResult<SpecId> {
            Ok(upgrade(&runtime, &ctx)?.push_node(build()))
        }),
    )
}

fn text_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    build: fn(String) -> Component,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(move |ctx: Ctx<'_>, value: String| -> JsResult<SpecId> {
            Ok(upgrade(&runtime, &ctx)?.push_node(build(value)))
        }),
    )
}

fn element_id(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<SpecId> {
    value
        .as_object()
        .and_then(|object| object.get::<_, u32>("__id").ok())
        .ok_or_else(|| {
            Exception::throw_type(ctx, "render(cx) must return an element built with gpui")
        })
}

/// The script-side `cx`. It carries only a generation; every use is checked
/// against the live scope stack, so a `cx` kept past its call reports clearly
/// instead of touching a dead frame.
fn context_object<'js>(ctx: &Ctx<'js>, generation: u64) -> JsResult<Object<'js>> {
    let object = Object::new(ctx.clone())?;

    object.set(
        "notify",
        Func::from(move |ctx: Ctx<'_>| -> JsResult<()> {
            let phase = scope::current_phase();
            if !phase.is_some_and(ScopePhase::allows_notify) {
                return Err(Exception::throw_type(
                    &ctx,
                    &format!(
                        "cx.notify() is not allowed during the `{}` phase; \
                         request a re-render from an event handler instead",
                        phase.map(ScopePhase::as_str).unwrap_or("none")
                    ),
                ));
            }

            let view = scope::current_view();
            scope::with_context(generation, move |_, app| {
                if let Some(view) = view {
                    // Two halves, and both matter. Invalidating says the script
                    // description may have moved, which is the only thing that
                    // lets the next frame enter the VM; notifying hands the
                    // scheduling and coalescing of that frame back to GPUI,
                    // which already does it well. Three notifies before the
                    // next frame therefore rebuild one snapshot, not three.
                    view.update(app, |view, cx| {
                        view.invalidate();
                        cx.notify();
                    });
                }
            })
            .map_err(|error| Exception::throw_type(&ctx, &error.to_string()))
        }),
    )?;

    overlay::install(ctx, &object, generation)?;

    object.set(
        "phase",
        Func::from(|| {
            scope::current_phase()
                .map(ScopePhase::as_str)
                .unwrap_or("none")
                .to_owned()
        }),
    )?;

    Ok(object)
}

/// One converted argument.
///
/// A JS closure cannot unify the `Ctx<'js>` lifetime with a `Vec<Value<'js>>`
/// parameter, so conversion happens inside `FromJs`, where both lifetimes are
/// still the same one. Handlers become `Persistent` here for the same reason.
enum Argument {
    Value(Bridged),
    Handler(Persistent<Function<'static>>),
}

struct Arguments(SmallVec<[Argument; 2]>);

impl Arguments {
    fn values(&self, method: &str) -> JsResult<SmallVec<[Bridged; 2]>> {
        self.0
            .iter()
            .map(|argument| match argument {
                Argument::Value(value) => Ok(value.clone()),
                Argument::Handler(_) => Err(JsError::new_from_js_message(
                    "function",
                    "value",
                    format!("`{method}` does not take a function"),
                )),
            })
            .collect()
    }

    fn first_value(&self) -> Option<&Bridged> {
        match self.0.first() {
            Some(Argument::Value(value)) => Some(value),
            _ => None,
        }
    }

    fn first_handler(&self) -> Option<Persistent<Function<'static>>> {
        match self.0.first() {
            Some(Argument::Handler(handler)) => Some(handler.clone()),
            _ => None,
        }
    }
}

impl<'js> FromJs<'js> for Arguments {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let array = value
            .into_array()
            .ok_or_else(|| Exception::throw_type(ctx, "expected an argument list"))?;

        let mut converted = SmallVec::new();
        for entry in array.iter::<Value>() {
            let entry = entry?;
            converted.push(if let Some(handler) = entry.as_function() {
                Argument::Handler(Persistent::save(ctx, handler.clone()))
            } else if entry.is_null() || entry.is_undefined() {
                Argument::Value(Bridged::Nil)
            } else if let Some(flag) = entry.as_bool() {
                Argument::Value(Bridged::Bool(flag))
            } else if let Some(number) = entry.as_number() {
                Argument::Value(Bridged::Number(number))
            } else if let Some(text) = entry.as_string() {
                Argument::Value(Bridged::Str(text.to_string()?))
            } else {
                return Err(Exception::throw_type(
                    ctx,
                    "unsupported argument type; expected null, boolean, number, string or function",
                ));
            });
        }

        Ok(Self(converted))
    }
}

fn unknown_method(name: &str) -> String {
    match style::suggest(name) {
        Some(candidate) => format!("unknown element method `{name}` (did you mean `{candidate}`?)"),
        None => format!(
            "unknown element method `{name}`; it is neither a style method nor one of \
             child, children, when, on_click, on_change, disabled, selected, checked"
        ),
    }
}

fn upgrade(runtime: &Weak<ShellRuntime>, ctx: &Ctx<'_>) -> JsResult<Rc<ShellRuntime>> {
    runtime
        .upgrade()
        .ok_or_else(|| Exception::throw_message(ctx, "the shell runtime has already shut down"))
}

/// Turns a QuickJS error into a message that includes the script's own stack,
/// which is the part an author actually needs.
fn describe(ctx: &Ctx<'_>, error: JsError) -> String {
    if !matches!(error, JsError::Exception) {
        return error.to_string();
    }
    let value = ctx.catch();
    match value.as_exception() {
        Some(exception) => match exception.stack() {
            Some(stack) => format!(
                "{}\n{stack}",
                exception.message().unwrap_or_else(|| "error".into())
            ),
            None => exception.message().unwrap_or_else(|| "error".into()),
        },
        None => format!("{value:?}"),
    }
}

fn js_setup_error(error: JsError) -> anyhow::Error {
    anyhow!("failed to start the JavaScript runtime: {error}")
}
