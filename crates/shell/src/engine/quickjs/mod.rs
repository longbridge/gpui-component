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
use gpui::{App, AppContext as _, ClickEvent, Entity, Global, WeakEntity, Window};
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
    policy::Policy,
    runtime::{ApplicationGeneration, CallbackArena, CallbackEntry},
    scope::{self, ScopePhase},
    snapshot::RenderSnapshot,
    spec::{CallbackId, Component, SpecArena, SpecId, SpecOp},
    style,
    value::Bridged,
    view::ScriptView,
};

const MAX_MODULE_BYTES: u64 = 8 * 1024 * 1024;

/// A script value that defines a view type — a JS class.
#[derive(Clone)]
pub struct ViewType {
    value: Persistent<Object<'static>>,
    module_lease: Option<ApplicationModuleLease>,
    application: Option<Rc<ApplicationGeneration>>,
}

impl std::fmt::Debug for ViewType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ViewType").finish_non_exhaustive()
    }
}

/// One instance of a view type.
#[derive(Clone)]
pub struct ViewObject {
    value: Persistent<Object<'static>>,
    #[allow(dead_code)] // Its drop owns the resolver registration lifetime.
    module_lease: Option<ApplicationModuleLease>,
    application: Option<Rc<ApplicationGeneration>>,
}

impl std::fmt::Debug for ViewObject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ViewObject").finish_non_exhaustive()
    }
}

impl ViewObject {
    fn unscoped(value: Persistent<Object<'static>>) -> Self {
        Self {
            value,
            module_lease: None,
            application: None,
        }
    }

    fn restore<'js>(self, ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
        self.value.restore(ctx)
    }

    pub(crate) fn application_generation(&self) -> Option<Rc<ApplicationGeneration>> {
        self.application.clone()
    }
}

mod entity_api;
pub(crate) mod host;
mod native;
mod overlay;
pub(crate) mod sandbox;
mod scheduler;

pub(crate) fn cancel_policy_tasks(policy: &Rc<Policy>) {
    scheduler::cancel_policy(policy);
}

pub(crate) fn cancel_application_tasks(generation: &Rc<ApplicationGeneration>) {
    scheduler::cancel_application_generation(generation);
}

#[cfg(test)]
pub(crate) fn task_count() -> usize {
    scheduler::task_count()
}

pub(super) struct InputCallbackOwner {
    policy: Rc<Policy>,
    application: Option<Rc<ApplicationGeneration>>,
    view: Option<WeakEntity<ScriptView>>,
}
mod standard;
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
    "image",
    "PathBuilder",
    "Background",
    "paint_path",
    "Button",
    "Link",
    "Checkbox",
    "Switch",
    "Tabs",
    "Tab",
    "Progress",
    "ProgressTrack",
    "ProgressIndicator",
    "fps_monitor",
    "Radio",
    "Toggle",
    "RadioGroup",
    "ToggleGroup",
    "Table",
    "TableHeader",
    "TableBody",
    "TableRow",
    "TableHead",
    "TableCell",
    "TableCaption",
    "h_resizable",
    "v_resizable",
    "resizable_panel",
    "Collapsible",
    "Popover",
    "HoverCard",
    "Popup",
    "Select",
    "Combobox",
    "DatePicker",
    "Scrollbar",
    "Input",
    "InputState",
    "NumberInput",
    "Textarea",
    "TextareaState",
    "SliderState",
    "Slider",
    "SliderTrack",
    "SliderIndicator",
    "SliderThumb",
    "FocusHandle",
    // System capabilities (`host`, `sandbox`).
    "store",
    "clipboard",
    "log",
    // Native modules (`native`).
    "native",
    // Theme (`theme_api`).
    "theme",
    "set_theme",
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
    /// An HTTP client supplied by tests that exercise a loopback server.
    ///
    /// This is deliberately runtime-scoped rather than process state: tests
    /// run concurrently, and changing proxy environment variables would leak
    /// into unrelated runtimes. Production builds do not carry this field and
    /// continue to construct the normal system-configured client in `fetch`.
    #[cfg(test)]
    test_http_client: RefCell<Option<reqwest::blocking::Client>>,
    context: JsContext,
    /// Incremented per `load_app`, so a reload re-reads every module rather
    /// than serving the first version from QuickJS's module cache.
    app_modules: AppModules,
    next_application_generation: Cell<u64>,
    /// Held so the context stays alive, and so the module loader can be scoped
    /// to an application directory when one is loaded.
    js_runtime: JsRuntime,
}

impl Drop for ShellRuntime {
    fn drop(&mut self) {
        // Both hold `Persistent` script values, and a persistent handle
        // released after its runtime aborts the process.
        scheduler::shutdown(self);
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
    /// Creates the application's default runtime and makes it available to
    /// shell callbacks registered on this [`App`].
    pub fn new(cx: &mut App) -> Result<Rc<Self>> {
        if Self::global(cx).is_some() {
            return Err(anyhow!(
                "a default gpui-shell runtime is already installed; use ShellRuntime::new_isolated() for an additional VM"
            ));
        }
        let runtime = Self::new_isolated()?;
        runtime.set_global(cx);
        Ok(runtime)
    }

    /// Creates a runtime without installing it as the application's default.
    ///
    /// More than one may be alive on a thread because authority travels on the
    /// call frame rather than in runtime-global state. Use this only when a host
    /// deliberately owns multiple isolated runtimes.
    pub fn new_isolated() -> Result<Rc<Self>> {
        let entities = EntityStore::try_new()
            .ok_or_else(|| anyhow!("gpui-shell entity store id space is exhausted"))?;
        let js_runtime = JsRuntime::new().map_err(js_setup_error)?;
        let context = JsContext::full(&js_runtime).map_err(js_setup_error)?;

        let app_modules = AppModules::default();
        js_runtime.set_loader(
            (
                standard::resolver(),
                BuiltinResolver::default().with_module("gpui"),
                app_modules.clone(),
            ),
            (
                standard::loader(),
                ModuleLoader::default().with_module("gpui", GpuiModule),
                app_modules.clone(),
            ),
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
            entities: RefCell::new(entities),
            metrics: Metrics::default(),
            #[cfg(test)]
            test_http_client: RefCell::new(None),
            context,
            app_modules,
            next_application_generation: Cell::new(1),
            js_runtime,
        });

        runtime.install_globals()?;
        Ok(runtime)
    }

    pub(crate) fn set_global(self: &Rc<Self>, cx: &mut App) {
        cx.set_global(RuntimeGlobal(self.clone()));
    }

    pub(crate) fn global(cx: &App) -> Option<Rc<Self>> {
        cx.try_global::<RuntimeGlobal>()
            .map(|global| global.0.clone())
    }

    /// What the runtime is spending: script renders and materializations, with
    /// the time each took.
    ///
    /// The two counters follow different things — application activity and
    /// frame count — and the gap between them is what the snapshot lifecycle
    /// exists to produce. See [`crate::metrics`].
    pub(crate) fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[cfg(test)]
    pub(crate) fn use_direct_http_for_tests(&self) {
        *self.test_http_client.borrow_mut() = Some(
            standard::direct_test_http_client()
                .expect("the direct test HTTP client should be constructible"),
        );
    }

    #[cfg(test)]
    fn test_http_client(&self) -> Option<reqwest::blocking::Client> {
        self.test_http_client.borrow().clone()
    }

    /// A reading of the two counters, taken now.
    ///
    /// The host gets the reading rather than the instrument: `Metrics` is the
    /// timing side, and a host holding it could reset the counters under a
    /// measurement someone else was taking. Subtract two readings with
    /// [`RuntimeMetrics::since`](crate::RuntimeMetrics::since) to measure an
    /// interval.
    pub fn read_metrics(&self) -> crate::metrics::RuntimeMetrics {
        self.metrics.read()
    }

    /// This runtime's retained state.
    ///
    /// Scoped to the runtime rather than shared, so one runtime cannot resolve
    /// another's handle — see [`crate::entities`].
    pub(crate) fn entities(&self) -> RefMut<'_, EntityStore> {
        self.entities.borrow_mut()
    }

    /// Loads `main.js` from an application directory.
    ///
    /// Module resolution is scoped to that directory: an application can import
    /// its own files and the built-in `gpui` module, and nothing else. That is
    /// the first half of the sandbox's module policy (design doc §19.1).
    pub fn load_app(self: &Rc<Self>, dir: &Path, entry: &str) -> Result<ViewType> {
        let root = crate::runtime::resolve_app_root(dir, entry)?;
        if let Err(error) = crate::write_type_declarations(&root) {
            tracing::debug!(
                "could not update declarations in {}: {error}",
                root.display()
            );
        }

        // Every load is a new generation, which is what makes a reload pick up
        // a change in an imported module rather than only in the entry point.
        let module_lease = self.app_modules.register(root.clone());
        let generation = module_lease.generation();
        let application = ApplicationGeneration::new(self.next_application_generation.get());
        self.next_application_generation.set(
            self.next_application_generation
                .get()
                .checked_add(1)
                .expect("a shell runtime exhausted its application generations"),
        );

        let entry = root.join(entry);
        let source = read_module_source(&entry)?;

        // The entry carries the generation too: it is a cached module like any
        // other, and a reload that re-read every import but served a stale
        // `main.js` would be the same bug one level up.
        let _application_scope = scope::enter_application(application.clone());
        let loaded = self.load_source_with_lease(
            &format!("{}?v={}", entry.to_string_lossy(), generation),
            &source,
            Some(module_lease),
            Some(application.clone()),
        );
        if loaded.is_err() {
            cancel_application_tasks(&application);
        }
        loaded
    }

    /// Evaluates a module and returns its default export, which must be a view
    /// class.
    pub fn load_source(self: &Rc<Self>, name: &str, source: &str) -> Result<ViewType> {
        self.load_source_with_lease(name, source, None, None)
    }

    fn load_source_with_lease(
        self: &Rc<Self>,
        name: &str,
        source: &str,
        module_lease: Option<ApplicationModuleLease>,
        application: Option<Rc<ApplicationGeneration>>,
    ) -> Result<ViewType> {
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
            Ok(ViewType {
                value: Persistent::save(ctx, class.clone()),
                module_lease,
                application,
            })
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
        let application = view_type.application.clone();
        let (_guard, _generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            None,
            crate::policy::default(),
            application.clone(),
        );
        let instance = match self.construct(view_type) {
            Ok(instance) => instance,
            Err(error) => {
                if let Some(application) = application {
                    cancel_application_tasks(&application);
                }
                return Err(error);
            }
        };
        if let Err(error) = self.initialize(&instance) {
            if let Some(application) = application {
                cancel_application_tasks(&application);
            }
            return Err(error);
        }
        Ok(instance)
    }

    /// Constructs and initializes a script view under its final owner.
    ///
    /// `init()` may start asynchronous work. Creating the GPUI entity first is
    /// what gives those tasks an owner, so a later `cx.notify()` can invalidate
    /// this view and dropping the view can cancel its work.
    pub fn instantiate_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>> {
        self.instantiate_view_with_policy(view_type, crate::policy::default(), window, cx)
    }

    pub fn instantiate_view_with_policy(
        self: &Rc<Self>,
        view_type: &ViewType,
        policy: Rc<crate::policy::Policy>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>> {
        let application = view_type.application.clone();
        let (construct_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            None,
            policy.clone(),
            application.clone(),
        );
        let object = match self.construct(view_type) {
            Ok(object) => object,
            Err(error) => {
                if let Some(application) = application {
                    cancel_application_tasks(&application);
                }
                return Err(error);
            }
        };
        drop(construct_scope);
        let view = cx.new(|_| ScriptView::with_policy(self.clone(), object, policy.clone()));
        let object = view.read(cx).object().clone();

        let (_initialize_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application.clone(),
        );
        if let Err(error) = self.initialize(&object) {
            if let Some(application) = application {
                cancel_application_tasks(&application);
            }
            return Err(error);
        }
        Ok(view)
    }

    pub fn instantiate_for_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        view: Entity<ScriptView>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ViewObject> {
        let policy = view.read(cx).policy();
        let application = view_type.application.clone();
        let (_guard, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view),
            policy,
            application.clone(),
        );
        let object = match self.construct(view_type) {
            Ok(object) => object,
            Err(error) => {
                if let Some(application) = application {
                    cancel_application_tasks(&application);
                }
                return Err(error);
            }
        };
        if let Err(error) = self.initialize(&object) {
            if let Some(application) = application {
                cancel_application_tasks(&application);
            }
            return Err(error);
        }
        Ok(object)
    }

    fn construct(&self, view_type: &ViewType) -> Result<ViewObject> {
        self.with_js(|ctx| {
            let class = view_type.value.clone().restore(ctx)?;
            let construct: Function = ctx.globals().get("__construct")?;
            let instance: Object = construct.call((class,))?;
            Ok(ViewObject {
                value: Persistent::save(ctx, instance),
                module_lease: view_type.module_lease.clone(),
                application: view_type.application.clone(),
            })
        })
    }

    fn initialize(&self, object: &ViewObject) -> Result<()> {
        self.with_js(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let initialize: Function = ctx.globals().get("__initialize")?;
            initialize.call::<_, ()>((instance,))
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
    pub(crate) fn build_snapshot(
        self: &Rc<Self>,
        object: &ViewObject,
        view: Option<Entity<ScriptView>>,
        policy: Rc<crate::policy::Policy>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<RenderSnapshot> {
        self.arena.borrow_mut().reset();
        let callbacks = self.callbacks.borrow_mut().begin();

        let (root, policy) = self.metrics.time_script_render(|| {
            let (_guard, generation) = scope::enter_with_application(
                self,
                window,
                cx,
                ScopePhase::Render,
                view.clone(),
                policy.clone(),
                object.application_generation(),
            );
            (self.call_render(object, generation), policy)
        });

        let root = match root {
            Ok(root) => root,
            Err(error) => {
                self.callbacks.borrow_mut().abort();
                self.arena.borrow_mut().reset();
                if let Some(view) = view {
                    scheduler::drain_after_render(self, view, policy, window, cx);
                }
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
            scheduler::drain_after_render(self, view, policy, window, cx);
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
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        Ok(self
            .build_snapshot(object, view, policy, window, cx)?
            .debug_tree())
    }

    /// Releases the handlers registered while one snapshot was built.
    ///
    /// Called by [`RenderSnapshot`] as it drops, which is what ties handler
    /// lifetime to snapshot lifetime rather than to a frame.
    pub(crate) fn retire_callbacks(&self, generation: u64) {
        self.callbacks.borrow_mut().retire(generation);
    }

    pub(crate) fn dispatch_click(
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

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("click callback {id} belongs to a retired application");
            return;
        }

        let policy = entry
            .view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            entry.view.clone(),
            policy,
            entry.application.clone(),
        );
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
        owner: &InputCallbackOwner,
        event: &gpui_base::input::InputEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_base::input::InputEvent;

        // Both owner and policy are captured when the script subscribes. The
        // input entity may outlive a view, so only a weak owner is retained; if
        // that owner is gone the callback may still run, but notify has no dead
        // view to keep alive or invalidate.
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("input callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

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

    /// Delivers a slider event to a long-lived script subscription.
    ///
    /// The value is the whole payload rather than a field of an object,
    /// because the value is the whole of what a slider event carries: one
    /// number, or the pair a two-thumbed slider moves between.
    pub(super) fn dispatch_slider_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        value: gpui_base::slider::SliderValue,
        window: &mut Window,
        cx: &mut App,
    ) {
        use gpui_base::slider::SliderValue;
        use rquickjs::IntoJs as _;

        // Captured when the script subscribed, for the same reason an input's
        // are: the state outlives any one view, so the grant a handler runs
        // under has to be the one it was registered with.
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("slider callback belongs to a retired application");
            return;
        }
        let view = owner.view.as_ref().and_then(WeakEntity::upgrade);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            owner.policy.clone(),
            owner.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = handler.clone().restore(ctx)?;
            let payload = match value {
                SliderValue::Single(value) => f64::from(value).into_js(ctx)?,
                SliderValue::Range(start, end) => {
                    vec![f64::from(start), f64::from(end)].into_js(ctx)?
                }
            };
            handler.call::<_, ()>((payload, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in slider handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Reports the panel sizes of a resizable group after a drag, in pixels and
    /// in the group's child order.
    ///
    /// Sizes are not state the script has to keep: base files them in window
    /// element state under the group's own id, so a drag survives every repaint
    /// that never enters the VM. This is a notification — persist it, mirror it
    /// into a title bar — and a group that ignores it still resizes.
    pub(crate) fn dispatch_resize(
        self: &Rc<Self>,
        id: CallbackId,
        sizes: Vec<f32>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("resize callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("resize callback {id} belongs to a retired application");
            return;
        }

        let policy = entry
            .view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            entry.view.clone(),
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = rquickjs::Array::new(ctx.clone())?;
            for (index, size) in sizes.iter().enumerate() {
                payload.set(index, *size)?;
            }
            handler.call::<_, ()>((payload, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in resize handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Controlled-value handlers report intent; the script stores the value and
    /// notifies. The host never mutates script state on its behalf.
    pub(crate) fn dispatch_change(
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

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("change callback {id} belongs to a retired application");
            return;
        }

        let policy = entry
            .view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            entry.view.clone(),
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((checked, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in change handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Reports which way a `NumberInput` stepped, by the two names base's
    /// `StepAction` carries.
    ///
    /// A string rather than a boolean, and not because two directions could not
    /// be one: `dispatch_change`'s `true` means "checked", and a handler reading
    /// `true` as "up" would be reading the wrong word. The script gets
    /// `"increment"` or `"decrement"`, which is what base calls them.
    pub(crate) fn dispatch_step(
        self: &Rc<Self>,
        id: CallbackId,
        action: &'static str,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("step callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("step callback {id} belongs to a retired application");
            return;
        }

        let policy = entry
            .view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            entry.view.clone(),
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((action, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in step handler: {error}");
        }
        scheduler::drain_jobs(&self.js_runtime);
    }

    /// Delivers a handler that reports only that something happened.
    ///
    /// `on_confirm` and `on_dismiss` have no value to carry: the combobox root
    /// they come from holds neither the options nor the selection, so the news
    /// is the action itself. The script still receives `(payload, cx)` with an
    /// empty payload, so every rendered handler has the same shape whether or
    /// not there was anything to put in it.
    pub(crate) fn dispatch_signal(
        self: &Rc<Self>,
        id: CallbackId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("signal callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("signal callback {id} belongs to a retired application");
            return;
        }

        let policy = entry
            .view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            entry.view.clone(),
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((Object::new(ctx.clone())?, context_object(ctx, generation)?))
        });

        if let Err(error) = result {
            tracing::error!("error in signal handler: {error}");
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
            let instance = object.value.clone().restore(ctx)?;
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
        sandbox::begin_host_execution();
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
            // Not a runtime state, but the same mechanism: a detached node
            // collecting ordinary style methods. A `SliderIndicator` draws its
            // filled part from this one.
            "range_style" => "range_style",
            other => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!(
                        "unknown state style `{other}`; expected hover, active, focus or \
                         range_style"
                    ),
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

    /// Fills a named element slot, detaching the element from the tree.
    ///
    /// The same `claim` a state style's declarations use: an element a
    /// component renders in a place of its own must not also be rendered among
    /// its children, and a script that tries to use it twice gets an error
    /// rather than a duplicate.
    fn fill_slot(&self, ctx: &Ctx<'_>, id: SpecId, name: &str, element: SpecId) -> JsResult<()> {
        let interned = match name {
            "content" => "content",
            "trigger" => "trigger",
            // A number input's three. Unlike the two above, none of them is
            // optional in practice: base's step buttons are unstyled, so an
            // undecorated one is invisible and unhittable, and the frame has no
            // editor of its own.
            "input" => "input",
            "decrement_button" => "decrement_button",
            "increment_button" => "increment_button",
            other => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!("unknown element slot `{other}`"),
                ));
            }
        };

        self.arena
            .borrow_mut()
            .claim(element)
            .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
        self.push_op_checked(ctx, self.push_op(id, SpecOp::Slot(interned, element)))
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
#[derive(Clone, Default)]
struct AppModules {
    applications: Rc<RefCell<Vec<ApplicationModules>>>,
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
    next_generation: Rc<Cell<u32>>,
}

#[derive(Clone)]
struct ApplicationModules {
    root: std::path::PathBuf,
    generation: u32,
}

#[derive(Clone)]
struct ApplicationModuleLease(Rc<ApplicationModuleRegistration>);

struct ApplicationModuleRegistration {
    applications: Rc<RefCell<Vec<ApplicationModules>>>,
    root: std::path::PathBuf,
    generation: u32,
}

impl ApplicationModuleLease {
    fn generation(&self) -> u32 {
        self.0.generation
    }
}

impl Drop for ApplicationModuleRegistration {
    fn drop(&mut self) {
        self.applications.borrow_mut().retain(|application| {
            application.root != self.root || application.generation != self.generation
        });
    }
}

impl AppModules {
    fn register(&self, root: std::path::PathBuf) -> ApplicationModuleLease {
        let generation = self.next_generation.get().wrapping_add(1);
        self.next_generation.set(generation);
        self.applications.borrow_mut().push(ApplicationModules {
            root: root.clone(),
            generation,
        });
        ApplicationModuleLease(Rc::new(ApplicationModuleRegistration {
            applications: self.applications.clone(),
            root,
            generation,
        }))
    }

    /// Strips the generation tag a resolved name carries.
    fn untag(name: &str) -> &str {
        name.split_once("?v=").map(|(path, _)| path).unwrap_or(name)
    }

    fn application_for_base(&self, base: &str) -> Option<ApplicationModules> {
        let generation = Self::generation(base)?;
        let base = Path::new(Self::untag(base));
        self.applications
            .borrow()
            .iter()
            .filter(|application| {
                application.generation == generation && base.starts_with(&application.root)
            })
            .max_by_key(|application| application.root.components().count())
            .cloned()
    }

    fn generation(name: &str) -> Option<u32> {
        name.rsplit_once("?v=")?.1.parse().ok()
    }

    #[cfg(test)]
    fn registration_count(&self) -> usize {
        self.applications.borrow().len()
    }

    fn candidate(
        &self,
        application: &ApplicationModules,
        base: &str,
        name: &str,
    ) -> Option<std::path::PathBuf> {
        let start = if name.starts_with('.') {
            Path::new(Self::untag(base)).parent()?.to_path_buf()
        } else {
            application.root.clone()
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
        let Some(application) = self.application_for_base(base) else {
            return Err(Exception::throw_message(
                ctx,
                &format!("cannot identify the application importing `{name}` from `{base}`"),
            ));
        };
        let Some(path) = self.candidate(&application, base, name) else {
            return Err(Exception::throw_message(
                ctx,
                &format!("cannot resolve module `{name}` from `{base}`"),
            ));
        };

        if !path.starts_with(&application.root) {
            return Err(Exception::throw_message(
                ctx,
                &format!(
                    "module `{name}` resolves outside the application directory `{}`",
                    application.root.display()
                ),
            ));
        }

        Ok(format!(
            "{}?v={}",
            path.to_string_lossy(),
            application.generation
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
        let source = read_module_source(Path::new(path))
            .map_err(|error| Exception::throw_message(ctx, &error.to_string()))?;
        Module::declare(ctx.clone(), name, source)
    }
}

fn read_module_source(path: &Path) -> Result<String> {
    use std::io::Read as _;

    let mut file =
        std::fs::File::open(path).with_context(|| format!("reading module {}", path.display()))?;
    let size = file.metadata()?.len();
    if size > MAX_MODULE_BYTES {
        anyhow::bail!(
            "module `{}` is {size} bytes, over the {MAX_MODULE_BYTES}-byte limit",
            path.display()
        );
    }
    let mut source = String::with_capacity(size as usize);
    file.by_ref()
        .take(MAX_MODULE_BYTES + 1)
        .read_to_string(&mut source)
        .with_context(|| format!("reading module {}", path.display()))?;
    if source.len() as u64 > MAX_MODULE_BYTES {
        anyhow::bail!(
            "module `{}` grew over the {MAX_MODULE_BYTES}-byte limit",
            path.display()
        );
    }
    Ok(source)
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
  // A named slot. The element is consumed exactly as `child` consumes one, so
  // it cannot also be added to the tree — which is the point: the component
  // renders it somewhere of its own, or not at all.
  const slot = (name) =>
    function (element) {
      __apply(this.__id, name, [element.__id]);
      return this;
    };

  methods.content = slot("content");
  methods.trigger = slot("trigger");

  // A number input's three. Every element carries them, as it carries `content`
  // and `trigger`, because one prototype is shared by all of them.
  methods.input = slot("input");
  methods.decrement_button = slot("decrement_button");
  methods.increment_button = slot("increment_button");

  // Focus is held by handle, so the element records the handle rather than the
  // wrapper object around it — the same unwrapping `Input.new(state)` does.
  methods.track_focus = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "track_focus(handle) expects a FocusHandle from FocusHandle.new(), not a name or an element",
      );
    }
    __apply(this.__id, "track_focus", [handle.__handle]);
    return this;
  };
  // The second handle a combobox root needs: the one the keyboard moves to when
  // the surface opens. Checked here for the same reason `track_focus` is — a
  // name or an element would otherwise be dropped on the Rust side and the
  // focus would simply never move.
  methods.content_focus_handle = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "content_focus_handle(handle) expects a FocusHandle from FocusHandle.new(), not a name or an element",
      );
    }
    __apply(this.__id, "content_focus_handle", [handle.__handle]);
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
  // Not a state: the filled part of a slider, declared the same way because
  // it is the same thing — a detached element collecting styles. The shell
  // positions the box; this says what it looks like.
  methods.range_style = state("range_style");

  const finiteNonNegative = (value, name) => {
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
      throw new TypeError(name + " must be a finite non-negative number");
    }
    return value;
  };

  const finitePositive = (value, name) => {
    if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
      throw new TypeError(name + " must be a finite positive number");
    }
    return value;
  };

  // A table index is one-based and whole. 0 and 1.5 are not values a screen
  // reader rounds off; they are cells announced in the wrong column, so they
  // are refused at the call site rather than cast quietly on the Rust side.
  const oneBased = (value, name) => {
    if (!Number.isInteger(value) || value < 1) {
      throw new TypeError(name + " must be a whole number of at least 1");
    }
    return value;
  };

  methods.transition = function (property, options) {
    property = String(property);
    if (!["opacity", "width", "height", "left", "top"].includes(property)) {
      throw new TypeError(
        "transition(property, policy) supports opacity, width, height, left or top; got " +
          JSON.stringify(property),
      );
    }
    const policy = typeof options === "number" ? { duration: options } : (options ?? {});
    const duration = finiteNonNegative(policy.duration ?? 0, "transition duration");
    const delay = finiteNonNegative(policy.delay ?? 0, "transition delay");
    const easing = policy.easing ?? "ease-out";
    if (!["linear", "ease-in", "ease-out", "ease-in-out"].includes(easing)) {
      throw new TypeError(
        "transition easing must be linear, ease-in, ease-out or ease-in-out; got " +
          JSON.stringify(easing),
      );
    }
    __apply(this.__id, "transition", [
      property,
      duration,
      delay,
      easing,
    ]);
    return this;
  };

  methods.spring = function (property, options) {
    property = String(property);
    if (!["opacity", "width", "height", "left", "top"].includes(property)) {
      throw new TypeError(
        "spring(property, policy) supports opacity, width, height, left or top; got " +
          JSON.stringify(property),
      );
    }
    const policy = options ?? {};
    const response = finiteNonNegative(policy.response ?? 250, "spring response");
    const damping = finiteNonNegative(policy.damping ?? 1, "spring damping");
    const epsilon = finitePositive(policy.epsilon ?? 0.001, "spring epsilon");
    __apply(this.__id, "spring", [
      property,
      response,
      damping,
      epsilon,
    ]);
    return this;
  };

  // Announced, not laid out: `axis` sets the semantic orientation of a
  // grouping container and never turns it into a row or a column. Checked here
  // so a typo reports at the call site instead of silently announcing the
  // container's default.
  methods.axis = function (value) {
    value = String(value);
    if (!["horizontal", "vertical"].includes(value)) {
      throw new TypeError(
        "axis(value) must be horizontal or vertical; got " + JSON.stringify(value),
      );
    }
    __apply(this.__id, "axis", [value]);
    return this;
  };

  // A bar's visibility policy. Unset follows the theme, which is what every
  // other scrollbar in the application does; the three named modes are checked
  // here so a typo reports at the call site instead of silently falling back.
  methods.mode = function (value) {
    value = String(value);
    if (!["scrolling", "hover", "always"].includes(value)) {
      throw new TypeError(
        "mode(value) must be scrolling, hover or always; got " + JSON.stringify(value),
      );
    }
    __apply(this.__id, "mode", [value]);
    return this;
  };

  // The content size a bar measures its thumb against. Both halves are
  // required: one axis sized by the script and the other by the scroll area is
  // a thumb that lies about one of them.
  methods.scroll_size = function (width, height) {
    __apply(this.__id, "scroll_size", [
      finiteNonNegative(width, "scroll_size width"),
      finiteNonNegative(height, "scroll_size height"),
    ]);
    return this;
  };

  // Which corner of an anchored surface is pinned to its trigger. The names
  // come from the host so that the check here, the parser behind it and the
  // union in gpui.d.ts cannot disagree. Checked at the call site because an
  // unrecognized anchor would otherwise open the surface in the component's
  // default corner, which looks like a positioning bug rather than a typo.
  methods.anchor = function (value) {
    value = String(value);
    if (!__anchorNames.includes(value)) {
      throw new TypeError(
        "anchor(value) must be one of " +
          __anchorNames.join(", ") +
          "; got " +
          JSON.stringify(value),
      );
    }
    __apply(this.__id, "anchor", [value]);
    return this;
  };

  // A popover opened by the wrong button is silence, not a visual mistake, so
  // an unknown button name is refused rather than falling back to the left one.
  methods.mouse_button = function (value) {
    value = String(value);
    if (!["left", "right", "middle"].includes(value)) {
      throw new TypeError(
        "mouse_button(value) must be left, right or middle; got " + JSON.stringify(value),
      );
    }
    __apply(this.__id, "mouse_button", [value]);
    return this;
  };

  // Milliseconds, as everywhere else a script names a duration.
  const delay = (name) =>
    function (ms) {
      __apply(this.__id, name, [finiteNonNegative(ms, name)]);
      return this;
    };

  methods.open_delay = delay("open_delay");
  methods.close_delay = delay("close_delay");

  // Two arguments rather than a range literal, which JavaScript has no spelling
  // for. The floor is required — a panel always has one, and base's own is
  // 100px — while the ceiling is optional, because most panels have none.
  methods.size_range = function (min, max) {
    const args = [finiteNonNegative(min, "size_range min")];
    if (max !== undefined && max !== null) {
      args.push(finiteNonNegative(max, "size_range max"));
    }
    __apply(this.__id, "size_range", args);
    return this;
  };

  // `size` and `visible` on a resizable panel are base's own inherent builders
  // — the initial size along the group's axis, and whether the panel is drawn —
  // and in Rust each shadows the `Styled` method of the same name for that one
  // type. Own properties on the panel object shadow the shared prototype by the
  // same mechanism, so a script writes what the Rust writes and `.size(200)`
  // still means width-and-height, `.visible()` still means `visibility`,
  // everywhere else.
  const resizablePanel = () => {
    const object = element(__resizable_panel());
    object.size = function (pixels) {
      __apply(this.__id, "panel_size", [finiteNonNegative(pixels, "resizable_panel size")]);
      return this;
    };
    object.visible = function (value) {
      __apply(this.__id, "panel_visible", [Boolean(value)]);
      return this;
    };
    return object;
  };

  const coordinate = (value, name) => {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && /^-?(?:\d+(?:\.\d*)?|\.\d+)%$/.test(value)) return value;
    throw new TypeError(name + " must be a finite pixel number or percentage string");
  };

  const background = (kind, values, opacityFactor = 1, colorSpace = "srgb") => Object.freeze({
    __background: true,
    kind,
    values: Object.freeze(values),
    opacityFactor,
    colorSpace,
    opacity(factor) {
      return background(kind, values, finiteNonNegative(factor, "background opacity"), colorSpace);
    },
    color_space(space) {
      space = String(space).toLowerCase();
      if (!['srgb', 'oklab'].includes(space)) throw new TypeError("background color_space must be srgb or oklab");
      return background(kind, values, opacityFactor, space);
    },
  });

  const asBackground = (value) => value?.__background
    ? value
    : background("solid", [String(value)]);

  const pathBuilder = (fill, width) => {
    const commands = [];
    const builder = {};
    const command = (name, arity, coordinateCount = arity) => (...args) => {
      if (args.length < arity) throw new TypeError(name + " expects at least " + arity + " argument(s)");
      for (let index = 0; index < coordinateCount; index++) coordinate(args[index], name + " coordinate");
      commands.push(Object.freeze([name, ...args]));
      return builder;
    };
    builder.move_to = command("move_to", 2);
    builder.line_to = command("line_to", 2);
    builder.curve_to = command("curve_to", 4);
    builder.cubic_bezier_to = command("cubic_bezier_to", 6);
    builder.arc_to = (...args) => {
      if (args.length < 7) throw new TypeError("arc_to expects at least 7 argument(s)");
      coordinate(args[0], "arc x radius");
      coordinate(args[1], "arc y radius");
      if (typeof args[2] !== "number" || !Number.isFinite(args[2])) throw new TypeError("arc rotation must be finite");
      coordinate(args[5], "arc destination x");
      coordinate(args[6], "arc destination y");
      commands.push(Object.freeze(["arc_to", ...args]));
      return builder;
    };
    builder.close = () => { commands.push(Object.freeze(["close"])); return builder; };
    builder.dash_array = (values) => {
      if (fill) throw new TypeError("dash_array is only available on stroke paths");
      if (!Array.isArray(values) || values.some((value) => typeof value !== "number" || !Number.isFinite(value) || value <= 0)) {
        throw new TypeError("dash_array(values) expects positive finite pixel numbers");
      }
      commands.push(Object.freeze(["dash_array", ...values]));
      return builder;
    };
    builder.add_polygon = (points, closed = true) => {
      if (!Array.isArray(points) || points.length === 0) throw new TypeError("add_polygon(points) expects a non-empty array");
      points.forEach((point, index) => {
        if (!Array.isArray(point) || point.length < 2) throw new TypeError("each polygon point must be [x, y]");
        command(index === 0 ? "move_to" : "line_to", 2)(point[0], point[1]);
      });
      if (closed) builder.close();
      return builder;
    };
    builder.build = () => Object.freeze({
      __path: true,
      fill,
      width,
      commands: Object.freeze(commands.slice()),
    });
    return builder;
  };

  const paintPath = (pathValue, paintValue) => {
    if (!pathValue?.__path) throw new TypeError("paint_path(path, background) expects a Path built by PathBuilder");
    const paint = asBackground(paintValue);
    const object = element(__path(
      pathValue.fill,
      paint.kind,
      paint.values.map(String).join("\u001f"),
      paint.opacityFactor,
      paint.colorSpace,
      pathValue.width,
    ));
    for (const [name, ...args] of pathValue.commands) __apply(object.__id, name, args);
    return object;
  };

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
    // What makes a text state a number state. There is no `NumberInputState`:
    // the step, the bounds and the mask are fields on this one, so a plain
    // input becomes a numeric one by being told about them.
    set_step: (step) => __input_set_step(handle, step === null || step === undefined ? null : Number(step)),
    set_min: (min) => __input_set_min(handle, min === null || min === undefined ? null : Number(min)),
    set_max: (max) => __input_set_max(handle, max === null || max === undefined ? null : Number(max)),
    set_masked: (masked) => __input_set_masked(handle, Boolean(masked)),
    set_loading: (loading) => __input_set_loading(handle, Boolean(loading)),
    release: () => __input_release(handle),
  });

  // The multi-line state shares almost all of its surface with the single-line
  // one, and adds the three calls that only mean anything once text can wrap.
  const textareaState = (handle) => ({
    __handle: handle,
    value: () => __textarea_value(handle),
    set_value: (next) => __textarea_set_value(handle, String(next ?? "")),
    on: (event, handler) => __textarea_on(handle, String(event), handler),
    set_rows: (rows) => __textarea_set_rows(handle, oneBased(rows, "set_rows(rows)")),
    set_auto_grow: (min_rows, max_rows) =>
      __textarea_set_auto_grow(
        handle,
        oneBased(min_rows, "set_auto_grow(min_rows, max_rows) min_rows"),
        oneBased(max_rows, "set_auto_grow(min_rows, max_rows) max_rows"),
      ),
    set_soft_wrap: (wrap) => __textarea_set_soft_wrap(handle, Boolean(wrap)),
    release: () => __textarea_release(handle),
  });

  // A slider's value crosses as an array either way, because a bare number
  // cannot say whether the script meant one thumb or two.
  const sliderValue = (values) => (values.length === 1 ? values[0] : values);
  const sliderValues = (value, api) => {
    const finite = (each) => typeof each === "number" && Number.isFinite(each);
    if (Array.isArray(value)) {
      if (value.length !== 2 || !value.every(finite)) {
        throw new TypeError(api + " expects a finite number, or a pair [start, end] of them");
      }
      return [value[0], value[1]];
    }
    if (!finite(value)) {
      throw new TypeError(api + " expects a finite number, or a pair [start, end] of them");
    }
    return [value];
  };

  const sliderState = (handle) => ({
    __handle: handle,
    value: () => sliderValue(__slider_value(handle)),
    set_value: (next) => __slider_set_value(handle, sliderValues(next, "set_value(value)")),
    min_value: () => __slider_bounds(handle)[0],
    max_value: () => __slider_bounds(handle)[1],
    step_value: () => __slider_bounds(handle)[2],
    on: (event, handler) => __slider_on(handle, String(event), handler),
    release: () => __slider_release(handle),
  });

  const focusHandle = (handle) => ({
    __handle: handle,
    focus: () => __focus_focus(handle),
    is_focused: () => __focus_is_focused(handle),
    release: () => __focus_release(handle),
  });

  let deferInit = false;
  globalThis.__construct = (Class) => {
    deferInit = true;
    try {
      return new Class();
    } finally {
      deferInit = false;
    }
  };
  globalThis.__initialize = (instance) => {
    if (typeof instance.init === "function") instance.init();
  };

  class View {
    constructor(props) {
      if (!deferInit && typeof this.init === "function") this.init(props);
    }
  }

  // A dialog and a sheet are views whose `render` is the author's function.
  // That is the whole of the wrapping: a script view is an object with a
  // `render`, so a content function already is one, once it is given the name.
  const contentView = (build, api) => {
    if (typeof build !== "function") {
      throw new TypeError(
        api + " takes a function returning an element, not an element and not a view class",
      );
    }
    return { render: () => build() };
  };

  // Overlays are window-level, not view-level: `cx.notify()` re-renders this
  // view, `window.open_dialog()` changes what the user is looking at. Grouped
  // under `window` because that is where `gpui-component` puts them — the
  // script API reads the same as the Rust it sits beside — and because it is
  // somewhere to grow: `Window` in Rust also answers focus, size and
  // appearance.
  //
  // A global rather than a module export, like `cx`. It names the window the
  // script is already inside, which is not something a file opts into by
  // importing it, and `window` is the one identifier every JavaScript author
  // already reaches for. Nothing collides: this runtime has no DOM.
  globalThis.window = {
    open_dialog: (build, options) =>
      __open_dialog(contentView(build, "window.open_dialog"), options ?? undefined),
    close_dialog: () => __close_dialog(),
    close_all_dialogs: () => __close_all_dialogs(),
    has_active_dialog: () => __has_active_dialog(),

    open_sheet: (build) => __open_sheet(undefined, contentView(build, "window.open_sheet")),
    open_sheet_at: (side, build) =>
      __open_sheet(String(side), contentView(build, "window.open_sheet_at")),
    close_sheet: () => __close_sheet(),
    has_active_sheet: () => __has_active_sheet(),

    push_toast: (options) => __push_toast(options),
    remove_toast: (id) => __remove_toast(String(id)),
    clear_toasts: () => __clear_toasts(),
  };

  let cachedThemeSource;
  let cachedTheme;
  const currentTheme = () => {
    const source = __theme_snapshot();
    if (source !== cachedThemeSource) {
      cachedThemeSource = source;
      cachedTheme = JSON.parse(source);
      Object.freeze(cachedTheme.colors);
      Object.freeze(cachedTheme.spacing);
      Object.freeze(cachedTheme.radius);
      Object.freeze(cachedTheme);
    }
    return cachedTheme;
  };

  const contextTheme = (check) => () => {
    check();
    return currentTheme();
  };

  return {
    View,
    div: () => element(__div()),
    h_flex: () => element(__h_flex()),
    v_flex: () => element(__v_flex()),
    text: (value) => element(__text(String(value))),
    svg: (path) => element(__svg(String(path))),
    image: (path) => element(__image(String(path))),
    paint_path: paintPath,
    PathBuilder: {
      fill: () => pathBuilder(true, 0),
      stroke: (width) => pathBuilder(false, finitePositive(width, "stroke width")),
    },
    Background: {
      solid: (color) => background("solid", [String(color)]),
      stop: (color, percentage) => {
        if (typeof percentage !== "number" || !Number.isFinite(percentage)) throw new TypeError("background stop percentage must be finite");
        return Object.freeze({ __backgroundStop: true, color: String(color), percentage });
      },
      linear_gradient: (angle, from, to) => {
        angle = Number(angle);
        if (!Number.isFinite(angle)) throw new TypeError("linear gradient angle must be finite");
        const stop = (value, fallback, name) => {
          if (typeof value === "string") return [value, fallback];
          if (!value?.__backgroundStop) throw new TypeError(name + " must be a color or Background.stop(color, percentage)");
          return [value.color, value.percentage];
        };
        const a = stop(from, 0, "gradient from stop");
        const b = stop(to, 1, "gradient to stop");
        return background("linear-gradient", [String(angle), a[0], String(a[1]), b[0], String(b[1])]);
      },
      pattern_slash: (color, width, interval) => background("pattern-slash", [
        String(color),
        String(finitePositive(width, "pattern width")),
        String(finitePositive(interval, "pattern interval")),
      ]),
      checkerboard: (color, size) => background("checkerboard", [
        String(color),
        String(finitePositive(size, "checkerboard size")),
      ]),
    },
    theme: currentTheme,
    __context_theme: contextTheme,
    Button: { new: (id) => element(__button(String(id))) },
    Link: { new: (id) => element(__link(String(id))) },
    Checkbox: { new: (id) => element(__checkbox(String(id))) },
    Switch: { new: (id) => element(__switch(String(id))) },
    Tabs: { new: (id) => element(__tabs(String(id))) },
    Tab: { new: (id) => element(__tab(String(id))) },
    Progress: { new: (id) => element(__progress(String(id))) },
    ProgressTrack: { new: () => element(__progress_track()) },
    ProgressIndicator: { new: () => element(__progress_indicator()) },
    fps_monitor: () => element(__fps_monitor()),
    Radio: { new: (id) => element(__radio(String(id))) },
    Toggle: { new: (id) => element(__toggle(String(id))) },
    RadioGroup: { new: (id) => element(__radio_group(String(id))) },
    ToggleGroup: { new: (id) => element(__toggle_group(String(id))) },
    Table: { new: (id) => element(__table(String(id))) },
    TableHeader: { new: (id) => element(__table_header(String(id))) },
    TableBody: { new: (id) => element(__table_body(String(id))) },
    TableCaption: { new: (id) => element(__table_caption(String(id))) },
    // Free functions, not `Type.new(...)`, because that is what base exports:
    // the group has no type a script ever names.
    h_resizable: (id) => element(__h_resizable(String(id))),
    v_resizable: (id) => element(__v_resizable(String(id))),
    resizable_panel: resizablePanel,
    TableRow: {
      new: (id, row_index) =>
        element(__table_row(String(id), oneBased(row_index, "TableRow.new row index"))),
    },
    TableHead: {
      new: (id, column_index) =>
        element(__table_head(String(id), oneBased(column_index, "TableHead.new column index"))),
    },
    TableCell: {
      new: (id, column_index) =>
        element(__table_cell(String(id), oneBased(column_index, "TableCell.new column index"))),
    },
    Collapsible: { new: () => element(__collapsible()) },
    Popover: { new: (id) => element(__popover(String(id))) },
    HoverCard: { new: (id) => element(__hover_card(String(id))) },
    // The trigger is a constructor argument, as it is in base: a popup with no
    // trigger has no bounds to anchor to, so there is no useful moment between
    // `new` and the trigger being known.
    Popup: {
      new: (id, trigger) => {
        if (typeof trigger?.__id !== "number") {
          throw new TypeError(
            "Popup.new(id, trigger) expects the trigger element; a popup anchors its content to the trigger's bounds, so it cannot be built without one",
          );
        }
        return element(__popup(String(id))).trigger(trigger);
      },
    },
    Select: { new: (id) => element(__select(String(id))) },
    Combobox: { new: (id) => element(__combobox(String(id))) },
    DatePicker: {
      new: (id, focus_handle) => {
        if (typeof focus_handle?.__handle !== "number") {
          throw new TypeError(
            "DatePicker.new(id, focus_handle) expects a FocusHandle from FocusHandle.new(); the picker takes the keyboard through that handle, and base has no builder to supply one later",
          );
        }
        return element(__date_picker(String(id), focus_handle.__handle));
      },
    },
    Scrollbar: {
      new: (id) => element(__scrollbar(String(id))),
      // `horizontal` and `vertical` are `new` plus the orientation the group
      // containers already spell `axis`, so there is one word for orientation
      // in the whole API rather than a second one here.
      horizontal: (id) => element(__scrollbar(String(id))).axis("horizontal"),
      vertical: (id) => element(__scrollbar(String(id))).axis("vertical"),
    },
    InputState: {
      new: (options) =>
        inputState(__input_state_new(options?.placeholder ?? null, options?.value ?? null)),
    },
    Input: { new: (state) => element(__input_element(state.__handle)) },

    NumberInput: {
      new: (state) => element(__number_input_element(state.__handle)),
    },
    TextareaState: {
      new: (options) =>
        textareaState(
          __textarea_state_new(
            options?.placeholder ?? null,
            options?.value ?? null,
            options?.rows === undefined || options?.rows === null
              ? null
              : oneBased(options.rows, "TextareaState.new rows"),
          ),
        ),
    },
    Textarea: { new: (state) => element(__textarea_element(state.__handle)) },
    SliderState: {
      new: (options) => {
        const settings = options ?? {};
        const min = settings.min ?? 0;
        const max = settings.max ?? 100;
        const step = settings.step ?? 1;
        const scale = String(settings.scale ?? "linear");
        for (const [name, value] of [["min", min], ["max", max], ["step", step]]) {
          if (typeof value !== "number" || !Number.isFinite(value)) {
            throw new TypeError("SliderState.new " + name + " must be a finite number");
          }
        }
        if (max <= min) throw new TypeError("SliderState.new needs a max greater than its min");
        if (step <= 0) throw new TypeError("SliderState.new step must be greater than 0");
        if (!["linear", "logarithmic"].includes(scale)) {
          throw new TypeError("SliderState.new scale must be linear or logarithmic");
        }
        // A logarithmic scale maps through log(value / min), which has no
        // answer at or below zero. Base asserts on it, and an assertion in the
        // host is a lost application rather than a reported mistake.
        if (scale === "logarithmic" && min <= 0) {
          throw new TypeError("SliderState.new with a logarithmic scale needs a min greater than 0");
        }
        return sliderState(
          __slider_state_new(
            min,
            max,
            step,
            scale,
            sliderValues(settings.value ?? min, "SliderState.new value"),
          ),
        );
      },
    },
    Slider: { new: (state) => element(__slider_element(state.__handle)) },
    SliderTrack: { new: (state) => element(__slider_track_element(state.__handle)) },
    SliderIndicator: { new: (state) => element(__slider_indicator_element(state.__handle)) },
    SliderThumb: { new: (state) => element(__slider_thumb_element(state.__handle)) },
    FocusHandle: { new: () => focusHandle(__focus_handle_new()) },
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
                "on_open_change",
                "on_confirm",
                "on_dismiss",
                "on_step",
                "disabled",
                "selected",
                "checked",
                "accessibility_label",
                "role",
                "aria_selected",
                "aria_active_descendant",
                "tab_index",
                "tab_stop",
                "href",
                "id",
                "overflow_scroll",
                "overflow_x_scroll",
                "overflow_y_scroll",
                "overflow_scrollbar",
                "overflow_x_scrollbar",
                "overflow_y_scrollbar",
                "viewport_from_layout",
                "controls_right",
                "on_resize",
                "set_position",
                "pressed",
                "start",
                "value",
                "indeterminate",
                "row_count",
                "column_count",
                "open",
                "default_open",
                "overlay_closable",
            ]
            .into_iter()
            .enumerate()
            {
                behaviors.set(index, name)?;
            }
            globals.set("__behaviorNames", behaviors)?;

            // The prelude checks an anchor at the call site, so it needs the
            // same eight names the parser accepts rather than a second copy of
            // them.
            let anchors = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in crate::materialize::ANCHOR_NAMES.into_iter().enumerate() {
                anchors.set(index, name)?;
            }
            globals.set("__anchorNames", anchors)?;

            constructor(&globals, "__div", runtime.clone(), || Component::Div)?;
            constructor(&globals, "__h_flex", runtime.clone(), || Component::HFlex)?;
            constructor(&globals, "__v_flex", runtime.clone(), || Component::VFlex)?;
            text_constructor(&globals, "__text", runtime.clone(), Component::Text)?;
            text_constructor(&globals, "__svg", runtime.clone(), Component::Svg)?;
            text_constructor(&globals, "__image", runtime.clone(), Component::Image)?;
            let path_runtime = runtime.clone();
            globals.set(
                "__path",
                Func::from(
                    move |ctx: Ctx<'_>,
                          fill: bool,
                          kind: String,
                          values: String,
                          opacity: f64,
                          color_space: String,
                          width: f64|
                          -> JsResult<SpecId> {
                        if !width.is_finite() || width < 0.0 {
                            return Err(Exception::throw_type(
                                &ctx,
                                "path stroke width must be finite and non-negative",
                            ));
                        }
                        if !opacity.is_finite() || opacity < 0.0 {
                            return Err(Exception::throw_type(
                                &ctx,
                                "path background opacity must be finite and non-negative",
                            ));
                        }
                        let values = values.split('\u{1f}').collect::<Vec<_>>();
                        let number = |index: usize, name: &str| -> JsResult<f32> {
                            values
                                .get(index)
                                .and_then(|value| value.parse::<f32>().ok())
                                .filter(|value| value.is_finite())
                                .ok_or_else(|| Exception::throw_type(&ctx, name))
                        };
                        let text = |index: usize, name: &str| -> JsResult<String> {
                            values
                                .get(index)
                                .map(|value| (*value).to_owned())
                                .ok_or_else(|| Exception::throw_type(&ctx, name))
                        };
                        let kind = match kind.as_str() {
                            "solid" => crate::spec::BackgroundKind::Solid {
                                color: text(0, "solid background needs a color")?,
                            },
                            "linear-gradient" => crate::spec::BackgroundKind::LinearGradient {
                                angle: number(0, "gradient angle must be finite")?,
                                from: (
                                    text(1, "gradient needs a from color")?,
                                    number(2, "gradient from percentage must be finite")?,
                                ),
                                to: (
                                    text(3, "gradient needs a to color")?,
                                    number(4, "gradient to percentage must be finite")?,
                                ),
                                color_space,
                            },
                            "pattern-slash" => crate::spec::BackgroundKind::PatternSlash {
                                color: text(0, "slash pattern needs a color")?,
                                width: number(1, "slash pattern width must be finite")?,
                                interval: number(2, "slash pattern interval must be finite")?,
                            },
                            "checkerboard" => crate::spec::BackgroundKind::Checkerboard {
                                color: text(0, "checkerboard needs a color")?,
                                size: number(1, "checkerboard size must be finite")?,
                            },
                            _ => {
                                return Err(Exception::throw_type(&ctx, "unknown Background kind"));
                            }
                        };
                        Ok(upgrade(&path_runtime, &ctx)?.push_node(Component::Path {
                            fill,
                            background: crate::spec::BackgroundSpec {
                                kind,
                                opacity: opacity as f32,
                            },
                            stroke_width: width as f32,
                        }))
                    },
                ),
            )?;
            text_constructor(&globals, "__button", runtime.clone(), Component::Button)?;
            text_constructor(&globals, "__link", runtime.clone(), Component::Link)?;
            text_constructor(&globals, "__checkbox", runtime.clone(), Component::Checkbox)?;
            text_constructor(&globals, "__switch", runtime.clone(), Component::Switch)?;
            text_constructor(
                &globals,
                "__scrollbar",
                runtime.clone(),
                Component::Scrollbar,
            )?;
            text_constructor(&globals, "__tabs", runtime.clone(), Component::Tabs)?;
            text_constructor(&globals, "__tab", runtime.clone(), Component::Tab)?;
            text_constructor(&globals, "__progress", runtime.clone(), Component::Progress)?;
            constructor(&globals, "__progress_track", runtime.clone(), || {
                Component::ProgressTrack
            })?;
            constructor(&globals, "__progress_indicator", runtime.clone(), || {
                Component::ProgressIndicator
            })?;
            constructor(&globals, "__fps_monitor", runtime.clone(), || {
                Component::FpsMonitor
            })?;
            text_constructor(&globals, "__radio", runtime.clone(), Component::Radio)?;
            text_constructor(&globals, "__toggle", runtime.clone(), Component::Toggle)?;
            text_constructor(
                &globals,
                "__radio_group",
                runtime.clone(),
                Component::RadioGroup,
            )?;
            text_constructor(
                &globals,
                "__toggle_group",
                runtime.clone(),
                Component::ToggleGroup,
            )?;
            text_constructor(&globals, "__table", runtime.clone(), Component::Table)?;
            text_constructor(
                &globals,
                "__table_header",
                runtime.clone(),
                Component::TableHeader,
            )?;
            text_constructor(
                &globals,
                "__table_body",
                runtime.clone(),
                Component::TableBody,
            )?;
            text_constructor(
                &globals,
                "__table_caption",
                runtime.clone(),
                Component::TableCaption,
            )?;
            indexed_constructor(
                &globals,
                "__table_row",
                runtime.clone(),
                Component::TableRow,
            )?;
            indexed_constructor(
                &globals,
                "__table_head",
                runtime.clone(),
                Component::TableHead,
            )?;
            indexed_constructor(
                &globals,
                "__table_cell",
                runtime.clone(),
                Component::TableCell,
            )?;
            // The axis comes from the constructor, so each one is a closure
            // over the variant rather than a second builder method.
            text_constructor(&globals, "__h_resizable", runtime.clone(), |id| {
                Component::Resizable(id, gpui::Axis::Horizontal)
            })?;
            text_constructor(&globals, "__v_resizable", runtime.clone(), |id| {
                Component::Resizable(id, gpui::Axis::Vertical)
            })?;
            constructor(&globals, "__resizable_panel", runtime.clone(), || {
                Component::ResizablePanel
            })?;
            constructor(&globals, "__collapsible", runtime.clone(), || {
                Component::Collapsible
            })?;
            text_constructor(&globals, "__popover", runtime.clone(), Component::Popover)?;
            text_constructor(
                &globals,
                "__hover_card",
                runtime.clone(),
                Component::HoverCard,
            )?;
            text_constructor(&globals, "__popup", runtime.clone(), Component::Popup)?;
            text_constructor(&globals, "__select", runtime.clone(), Component::Select)?;
            text_constructor(&globals, "__combobox", runtime.clone(), Component::Combobox)?;

            // The one constructor that takes retained state as well as an id.
            // Base's `DatePicker::new` requires the focus handle, so a picker
            // whose handle has already been released is refused where it was
            // written rather than rendered as an unreachable trigger.
            let date_picker_runtime = runtime.clone();
            globals.set(
                "__date_picker",
                Func::from(
                    move |ctx: Ctx<'_>,
                          id: String,
                          handle: crate::entities::EntityHandle|
                          -> JsResult<SpecId> {
                        let store = upgrade(&date_picker_runtime, &ctx)?;
                        if store.entities().focus(handle).is_none() {
                            return Err(Exception::throw_type(
                                &ctx,
                                "the focus handle given to DatePicker.new(id, focus_handle) has \
                                 been released; a date picker takes the keyboard through that \
                                 handle, so it needs a live one",
                            ));
                        }
                        Ok(store.push_node(Component::DatePicker(id, handle)))
                    },
                ),
            )?;

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

            // Before the prelude, which builds the `window` object over these.
            overlay::install(ctx, &ctx.globals())?;

            ctx.eval::<(), _>(PRELUDE)?;

            // Subsystems extend the same module object the prelude built.
            let module: Object = ctx.globals().get("__gpui")?;
            host::install(ctx, &module)?;
            native::install(ctx, &module)?;
            theme_api::install(ctx, &module)?;
            entity_api::install(ctx, &module, runtime.clone())?;
            scheduler::install(ctx, &module)?;
            // Standard Runtime constructors and prototypes must exist before
            // the sandbox freezes built-ins, or they would remain mutable.
            standard::install(ctx)?;
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
                // A `resizable_panel()` is not an element anywhere else:
                // base's panel reads its size out of the group's state and
                // panics outright without one. Refused here, where the script
                // can be pointed at the line that did it, rather than at paint
                // time.
                let orphan = {
                    let arena = self.arena.borrow();
                    let component = |node| {
                        arena
                            .node(node)
                            .and_then(crate::spec::SpecNode::component)
                            .cloned()
                    };
                    matches!(component(child), Some(Component::ResizablePanel))
                        && !matches!(component(id), Some(Component::Resizable(..)))
                };
                if orphan {
                    return Err(Exception::throw_type(
                        ctx,
                        "resizable_panel() belongs to an h_resizable() or v_resizable(): its size \
                         and its drag handle are the group's. Use a div() here instead",
                    ));
                }
                let attached = self.arena.borrow_mut().attach(id, child);
                self.push_op_checked(ctx, attached)
            }
            "content" | "trigger" | "input" | "decrement_button" | "increment_button" => {
                let element = args
                    .first_value()
                    .and_then(|value| value.as_f32().ok())
                    .ok_or_else(|| {
                        Exception::throw_type(ctx, &format!("{method}(element) expects an element"))
                    })? as SpecId;
                self.fill_slot(ctx, id, method, element)
            }
            "on_click" | "on_resize" | "on_change" | "on_open_change" | "on_confirm"
            | "on_dismiss" | "on_step" => {
                let saved = args.first_handler().ok_or_else(|| {
                    Exception::throw_type(ctx, &format!("{method}(handler) expects a function"))
                })?;
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: saved,
                    view: scope::current_view(),
                    application: scope::current_application_generation(),
                });
                let name = match method {
                    "on_click" => "on_click",
                    "on_resize" => "on_resize",
                    "on_change" => "on_change",
                    "on_confirm" => "on_confirm",
                    "on_dismiss" => "on_dismiss",
                    "on_step" => "on_step",
                    _ => "on_open_change",
                };
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Callback(name, callback)))
            }
            "disabled"
            | "selected"
            | "checked"
            | "accessibility_label"
            | "role"
            | "aria_selected"
            | "aria_active_descendant"
            | "track_focus"
            | "content_focus_handle"
            | "tab_index"
            | "tab_stop"
            | "href"
            | "id"
            | "overflow_scroll"
            | "overflow_x_scroll"
            | "overflow_y_scroll"
            | "overflow_scrollbar"
            | "overflow_x_scrollbar"
            | "overflow_y_scrollbar"
            | "mode"
            | "scroll_size"
            | "viewport_from_layout"
            | "controls_right"
            | "panel_visible"
            | "panel_size"
            | "size_range"
            | "set_position"
            | "pressed"
            | "start"
            | "value"
            | "indeterminate"
            | "axis"
            | "row_count"
            | "column_count"
            | "open"
            | "default_open"
            | "overlay_closable"
            | "anchor"
            | "mouse_button"
            | "open_delay"
            | "close_delay"
            | "transition"
            | "spring" => {
                let bridged = args.values(method)?;
                let name = match method {
                    "disabled" => "disabled",
                    "selected" => "selected",
                    "checked" => "checked",
                    "role" => "role",
                    "aria_selected" => "aria_selected",
                    "aria_active_descendant" => "aria_active_descendant",
                    "track_focus" => "track_focus",
                    "content_focus_handle" => "content_focus_handle",
                    "tab_index" => "tab_index",
                    "tab_stop" => "tab_stop",
                    "id" => "id",
                    "overflow_scroll" => "overflow_scroll",
                    "overflow_x_scroll" => "overflow_x_scroll",
                    "overflow_y_scroll" => "overflow_y_scroll",
                    "overflow_scrollbar" => "overflow_scrollbar",
                    "overflow_x_scrollbar" => "overflow_x_scrollbar",
                    "overflow_y_scrollbar" => "overflow_y_scrollbar",
                    "mode" => "mode",
                    "scroll_size" => "scroll_size",
                    "viewport_from_layout" => "viewport_from_layout",
                    "controls_right" => "controls_right",
                    "panel_visible" => "panel_visible",
                    "panel_size" => "panel_size",
                    "size_range" => "size_range",
                    "set_position" => "set_position",
                    "pressed" => "pressed",
                    "start" => "start",
                    "value" => "value",
                    "indeterminate" => "indeterminate",
                    "axis" => "axis",
                    "row_count" => "row_count",
                    "column_count" => "column_count",
                    "open" => "open",
                    "default_open" => "default_open",
                    "overlay_closable" => "overlay_closable",
                    "anchor" => "anchor",
                    "mouse_button" => "mouse_button",
                    "open_delay" => "open_delay",
                    "close_delay" => "close_delay",
                    "transition" => "transition",
                    "spring" => "spring",
                    "href" => "href",
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
                // A bar that silently sits at zero because the percentage
                // arrived as a string is the kind of bug that gets blamed on
                // the layout. Say it at the call site instead.
                if name == "value" && bridged.first().and_then(finite_number).is_none() {
                    return Err(Exception::throw_type(
                        ctx,
                        "value(percent) expects a number between 0 and 100",
                    ));
                }
                if name == "set_position" {
                    let position = bridged.first().and_then(finite_whole_number);
                    let size = bridged.get(1).and_then(finite_whole_number);
                    if !matches!((position, size), (Some(position), Some(size)) if position >= 1.0 && size >= position && size <= usize::MAX as f32)
                    {
                        return Err(Exception::throw_type(
                            ctx,
                            "set_position(position, size) expects whole finite numbers with 1 <= position <= size",
                        ));
                    }
                }
                if matches!(name, "row_count" | "column_count")
                    && !bridged
                        .first()
                        .and_then(finite_whole_number)
                        .is_some_and(|count| count >= 0.0 && count <= usize::MAX as f32)
                {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!("{name}(count) expects a non-negative whole finite number"),
                    ));
                }
                // An unknown role is silence in the accessibility tree, and
                // silence is exactly what `role` was called to prevent. The
                // one filtered variant is named separately, because "unknown
                // role" is not the answer to a script that asked for it.
                if name == "role" {
                    let Some(named) = bridged.first().and_then(|value| value.as_str().ok()) else {
                        return Err(Exception::throw_type(
                            ctx,
                            "role(name) expects a string; see the Role type in gpui.d.ts",
                        ));
                    };
                    if named == crate::a11y::FILTERED_ROLE {
                        return Err(Exception::throw_type(
                            ctx,
                            "role(\"generic_container\") announces nothing: GPUI filters that \
                             role out of the accessibility tree. Leave the role off instead, \
                             or name the role the element really has",
                        ));
                    }
                    if crate::a11y::role_from_name(named).is_none() {
                        return Err(Exception::throw_type(
                            ctx,
                            &format!(
                                "unknown accessibility role `{named}`; the names mirror \
                                 gpui::Role in snake_case — see the Role type in gpui.d.ts"
                            ),
                        ));
                    }
                }
                // A tab index of 1.5 is not a position in the tab order; it is
                // a number the script computed wrongly, and rounding it here
                // would put the control somewhere nobody chose.
                if name == "tab_index"
                    && !bridged
                        .first()
                        .and_then(|value| value.as_f32().ok())
                        .is_some_and(|index| index.fract() == 0.0)
                {
                    return Err(Exception::throw_type(
                        ctx,
                        "tab_index(index) expects a whole number",
                    ));
                }
                if name == "href" {
                    let Some(target) = bridged.first().and_then(|value| value.as_str().ok()) else {
                        return Err(Exception::throw_type(ctx, "href(url) expects a string"));
                    };
                    let valid = reqwest::Url::parse(target).is_ok_and(|url| {
                        matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
                    });
                    if !valid {
                        return Err(Exception::throw_type(
                            ctx,
                            "href(url) expects an absolute HTTP(S) URL with a host",
                        ));
                    }
                }
                self.push_op_checked(ctx, self.push_op(id, SpecOp::Method(name, bridged)))
            }
            "move_to" | "line_to" | "curve_to" | "cubic_bezier_to" | "arc_to" | "close"
            | "dash_array" => {
                let bridged = args.values(method)?;
                let name = match method {
                    "move_to" => "move_to",
                    "line_to" => "line_to",
                    "curve_to" => "curve_to",
                    "cubic_bezier_to" => "cubic_bezier_to",
                    "arc_to" => "arc_to",
                    "close" => "close",
                    _ => "dash_array",
                };
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

fn finite_number(value: &Bridged) -> Option<f32> {
    value.as_f32().ok().filter(|number| number.is_finite())
}

fn finite_whole_number(value: &Bridged) -> Option<f32> {
    finite_number(value).filter(|number| number.fract() == 0.0)
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

/// A constructor whose second argument is a one-based accessibility index.
///
/// `TableRow::new(id, row_index)` and the two cell types take their index in
/// the constructor rather than through a builder, because a cell that does not
/// know its column is not merely unstyled — it announces itself in the wrong
/// place. The script side refuses anything that is not a whole number of at
/// least one, so the cast here cannot quietly floor a fraction into a
/// plausible-looking index.
fn indexed_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    build: fn(String, usize) -> Component,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(
            move |ctx: Ctx<'_>, value: String, index: usize| -> JsResult<SpecId> {
                Ok(upgrade(&runtime, &ctx)?.push_node(build(value, index)))
            },
        ),
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

    let module: Object = ctx.globals().get("__gpui")?;
    let context_theme: Function = module.get("__context_theme")?;
    let check = Func::from(move |ctx: Ctx<'_>| -> JsResult<()> {
        scope::with_context(generation, |_, _| ())
            .map_err(|error| Exception::throw_type(&ctx, &error.to_string()))
    });
    let theme: Function = context_theme.call((check,))?;
    object.set("theme", theme)?;

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
             child, children, when, on_click, on_change, disabled, selected, checked, \
             overflow_scroll, overflow_x_scroll, overflow_y_scroll, overflow_scrollbar, \
             overflow_x_scrollbar, overflow_y_scrollbar"
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

#[cfg(test)]
mod module_lifecycle_tests {
    use super::{AppModules, ShellRuntime};

    #[test]
    fn registrations_for_the_same_root_are_generation_scoped_and_leased() {
        let modules = AppModules::default();
        let root = std::env::temp_dir().join("gpui-shell-module-lifecycle");

        let first = modules.register(root.clone());
        let second = modules.register(root.clone());

        assert_ne!(first.generation(), second.generation());
        assert_eq!(modules.registration_count(), 2);

        let retained = first.clone();
        drop(first);
        assert_eq!(modules.registration_count(), 2);
        drop(retained);
        assert_eq!(modules.registration_count(), 1);
        drop(second);
        assert_eq!(modules.registration_count(), 0);
    }

    #[test]
    fn importer_tags_select_the_exact_same_root_generation() {
        let modules = AppModules::default();
        let root = std::env::temp_dir().join("gpui-shell-module-generation");
        let first = modules.register(root.clone());
        let second = modules.register(root.clone());

        let first_importer = format!("{}/main.js?v={}", root.display(), first.generation());
        let second_importer = format!("{}/main.js?v={}", root.display(), second.generation());

        assert_eq!(
            modules
                .application_for_base(&first_importer)
                .expect("first generation")
                .generation,
            first.generation()
        );
        assert_eq!(
            modules
                .application_for_base(&second_importer)
                .expect("second generation")
                .generation,
            second.generation()
        );
    }

    #[test]
    fn an_older_same_root_class_keeps_its_import_generation() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-same-root-generation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("application directory");
        std::fs::write(
            root.join("main.js"),
            "import './feature.js';\n\
             export default class Panel {\n\
               static async label() { return (await import('./feature.js')).label; }\n\
             }",
        )
        .expect("entry module");
        std::fs::write(root.join("feature.js"), "export const label = 'first';")
            .expect("first feature");

        let first = runtime.load_app(&root, "main.js").expect("first load");
        std::fs::write(root.join("feature.js"), "export const label = 'second';")
            .expect("second feature");
        let second = runtime.load_app(&root, "main.js").expect("second load");

        let label = |view_type: &super::ViewType| {
            runtime
                .with_js(|ctx| {
                    let class = view_type.value.clone().restore(ctx)?;
                    let label: rquickjs::Function = class.get("label")?;
                    label.call::<_, rquickjs::Promise>(())?.finish::<String>()
                })
                .expect("dynamic import")
        };
        assert_eq!(label(&first), "first");
        assert_eq!(label(&second), "second");

        drop(first);
        assert_eq!(runtime.app_modules.registration_count(), 1);
        drop(second);
        assert_eq!(runtime.app_modules.registration_count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_failed_load_releases_its_module_registration() {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-failed-module-generation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("application directory");
        std::fs::write(
            root.join("main.js"),
            "import './missing.js'; export default class Panel {}",
        )
        .expect("entry module");

        runtime
            .load_app(&root, "main.js")
            .expect_err("missing import must reject the load");
        assert_eq!(runtime.app_modules.registration_count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }
}
