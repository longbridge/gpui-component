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
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    rc::{Rc, Weak},
};

use anyhow::{Context as _, Result, anyhow};
use gpui::{App, AppContext as _, ClickEvent, Entity, Global, WeakEntity, Window};
use rquickjs::{
    Array, Context as JsContext, Ctx, Error as JsError, Exception, FromJs, Function, Object,
    Persistent, Result as JsResult, Runtime as JsRuntime, Value,
    function::{Func, Opt, This},
    loader::{BuiltinResolver, ImportAttributes, Loader, ModuleLoader, Resolver},
    module::Declared,
    module::{Declarations, Exports, Module, ModuleDef},
};
use smallvec::SmallVec;

use crate::{
    entities::{EntityHandle, EntityStore},
    metrics::Metrics,
    policy::Policy,
    runtime::{ApplicationGeneration, CallbackArena, CallbackEntry},
    scope::{self, ScopePhase},
    snapshot::RenderSnapshot,
    spec::{CallbackId, ChildViewSpec, Component, SpecArena, SpecId, SpecOp},
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

/// A class or props value captured by a host function without leaking the
/// active QuickJS lifetime into the Rust callback type.
struct NestedViewClass(Persistent<Object<'static>>);

impl<'js> FromJs<'js> for NestedViewClass {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let class = value.as_object().ok_or_else(|| {
            Exception::throw_type(ctx, "cx.new(Class, props) expects a View subclass")
        })?;
        Ok(Self(Persistent::save(ctx, class.clone())))
    }
}

struct NestedViewProps(Persistent<Value<'static>>);

impl<'js> FromJs<'js> for NestedViewProps {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(Persistent::save(ctx, value)))
    }
}

struct ViewStateCheckpoint(Persistent<Function<'static>>);

#[derive(Clone)]
struct NestedViewProvenance {
    application: Option<Rc<ApplicationGeneration>>,
    policy: Rc<Policy>,
}

impl NestedViewProvenance {
    fn is_current(&self) -> bool {
        let Some(policy) = scope::current_policy() else {
            return false;
        };
        if !Rc::ptr_eq(&self.policy, &policy) {
            return false;
        }
        match (&self.application, scope::current_application_generation()) {
            (Some(expected), Some(current)) => {
                expected.is_active() && Rc::ptr_eq(expected, &current)
            }
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
struct NestedViewAlias {
    handle: EntityHandle,
    provenance: NestedViewProvenance,
}

/// A synchronous-looking script operation deferred until the active
/// `Context::with` entry has returned. This is what lets the implementation
/// reuse the ordinary, non-reentrant transactional job drains.
enum PendingNestedOperation {
    Create {
        runtime: Weak<ShellRuntime>,
        token: u32,
        owner: Entity<ScriptView>,
        view_type: ViewType,
        policy: Rc<crate::policy::Policy>,
        props: Persistent<Value<'static>>,
    },
    Update {
        runtime: Weak<ShellRuntime>,
        token: u32,
        provenance: NestedViewProvenance,
        props: Persistent<Value<'static>>,
    },
    Release {
        runtime: Weak<ShellRuntime>,
        token: u32,
        provenance: NestedViewProvenance,
    },
}

struct NestedFlushGuard<'a>(&'a Cell<bool>);

impl Drop for NestedFlushGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
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

pub(crate) fn cancel_view_tasks(runtime: &Rc<ShellRuntime>, entity_id: gpui::EntityId) {
    scheduler::cancel_view(runtime, entity_id);
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

/// The names each built-in module exports.
///
/// One module per crate that provides the capability, so an import says which
/// layer a script depends on: `gpui-base`'s components come from `"gpui-base"`,
/// `gpui-fps`'s overlay from `"gpui-fps"`, and `"gpui"` carries only what GPUI
/// itself and this runtime provide. A name belongs to exactly one of them —
/// nothing is re-exported for convenience, because a name reachable from two
/// specifiers stops saying anything about where it came from, and the next
/// layer to arrive would have to be told apart from the ones already here.
///
/// Anything installed onto `globalThis.__gpui` must be listed in one of these
/// or no `import { … }` will see it.
pub(crate) mod exports {
    /// GPUI's own elements and this runtime's script surface.
    pub(crate) const GPUI: &[&str] = &[
        // Views (`ScriptView`).
        "View",
        // Elements GPUI itself draws.
        "div",
        "svg",
        "image",
        "PathBuilder",
        "Background",
        // System capabilities (`host`, `sandbox`). Diagnostics are JavaScript's
        // own `console`; a second name for it bought nothing.
        "store",
        // Native modules (`native`).
        "native",
    ];

    /// Components, layout helpers and the theme, all owned by `gpui-base`.
    pub(crate) const GPUI_BASE: &[&str] = &[
        // Layout.
        "h_flex",
        "v_flex",
        // Controls.
        "Button",
        "Link",
        "Checkbox",
        "Switch",
        "Tabs",
        "Tab",
        "Progress",
        "ProgressTrack",
        "ProgressIndicator",
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
        "v_virtual_list",
        "h_virtual_list",
        "VirtualListScrollHandle",
        // Text editing.
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
        "OtpState",
        "OtpInput",
        // Theme (`theme_api`). Reading is `cx.theme()`; replacing the whole
        // palette is an application-level act with no context to speak of.
        "set_theme",
    ];

    /// The performance overlay, owned by `gpui-fps`.
    pub(crate) const GPUI_FPS: &[&str] = &["fps_monitor"];
}

/// Defines one `ModuleDef` per built-in module and the loader wiring for all of
/// them, so adding a layer — `gpui-component`, when its components arrive — is
/// a list and a line rather than another copy of the same three impls.
///
/// Every module re-exports values that were built at startup and stashed on
/// `globalThis.__gpui`; the split is in what each one names, not in where the
/// values live.
macro_rules! builtin_modules {
    ($(($module:ident, $specifier:literal, $names:expr)),+ $(,)?) => {
        $(
            struct $module;

            impl $module {
                const SPECIFIER: &'static str = $specifier;
                const NAMES: &'static [&'static str] = $names;
            }

            impl ModuleDef for $module {
                fn declare(declarations: &Declarations) -> JsResult<()> {
                    for name in Self::NAMES {
                        declarations.declare(*name)?;
                    }
                    Ok(())
                }

                fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> JsResult<()> {
                    let module: Object = ctx.globals().get("__gpui")?;
                    for name in Self::NAMES {
                        let value: Value = module.get(*name)?;
                        exports.export(*name, value)?;
                    }
                    Ok(())
                }
            }
        )+

        /// The specifiers a script may import, and nothing else.
        fn builtin_resolver() -> BuiltinResolver {
            BuiltinResolver::default()$(.with_module($module::SPECIFIER))+
        }

        /// Named in the refusal when a bare specifier is not one of them, so a
        /// script written against a different runtime than the one running it
        /// is told which it is talking to rather than only that the import
        /// failed.
        fn builtin_specifiers() -> String {
            [$($module::SPECIFIER),+]
                .map(|specifier| format!("`{specifier}`"))
                .join(", ")
        }

        fn builtin_loader() -> ModuleLoader {
            ModuleLoader::default()$(.with_module($module::SPECIFIER, $module))+
        }

        /// Which module exports `name`, if any.
        #[cfg(test)]
        fn module_exporting(name: &str) -> Option<&'static str> {
            $(
                if $module::NAMES.contains(&name) {
                    return Some($module::SPECIFIER);
                }
            )+
            None
        }
    };
}

builtin_modules![
    (GpuiModule, "gpui", exports::GPUI),
    (GpuiBaseModule, "gpui-base", exports::GPUI_BASE),
    (GpuiFpsModule, "gpui-fps", exports::GPUI_FPS),
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
    /// Operations requested by a native function are applied immediately
    /// after the enclosing QuickJS entry unlocks the context. The queue is
    /// declared before `context` because it owns persistent JS values.
    pending_nested: RefCell<VecDeque<PendingNestedOperation>>,
    flushing_nested: Cell<bool>,
    in_flight_nested: RefCell<HashMap<u32, NestedViewProvenance>>,
    initializing_views: RefCell<Vec<ViewObject>>,
    nested_view_handles: RefCell<HashMap<u32, NestedViewAlias>>,
    next_nested_view_token: Cell<u32>,
    /// A runtime whose opaque QuickJS job queue could not reach an ownership
    /// boundary safely is never entered again. QuickJS exposes no selective
    /// pending-job removal, so terminal quarantine is the only way to prevent
    /// the unfinished wave from later running under another view.
    terminal_job_error: RefCell<Option<String>>,
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

/// The App can find its default runtime without becoming its owner.
///
/// Shell views and host state own runtime lifetime. Once they are gone,
/// `ShellRuntime::new` can replace this expired registration naturally.
struct RuntimeGlobal(Weak<ShellRuntime>);

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
                builtin_resolver(),
                app_modules.clone(),
            ),
            (standard::loader(), builtin_loader(), app_modules.clone()),
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
            pending_nested: RefCell::new(VecDeque::new()),
            flushing_nested: Cell::new(false),
            in_flight_nested: RefCell::new(HashMap::new()),
            initializing_views: RefCell::new(Vec::new()),
            nested_view_handles: RefCell::new(HashMap::new()),
            next_nested_view_token: Cell::new(0),
            terminal_job_error: RefCell::new(None),
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
        cx.set_global(RuntimeGlobal(Rc::downgrade(self)));
    }

    pub(crate) fn global(cx: &App) -> Option<Rc<Self>> {
        cx.try_global::<RuntimeGlobal>()
            .and_then(|global| global.0.upgrade())
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

    /// Evaluates a fragment of script in this runtime's context.
    ///
    /// Test-only, and used by one caller: `tests::benchmark` has to time a loop
    /// of bare `__apply` calls to separate the cost of crossing the language
    /// boundary from the cost of what happens on the far side, and a view's
    /// `render()` cannot express that loop without the surrounding element
    /// construction being part of the measurement.
    #[cfg(test)]
    pub(crate) fn eval_for_benchmark(&self, source: &str) -> Result<()> {
        self.with_js(|ctx| ctx.eval::<(), _>(source))
    }

    /// Empties the scratch arena between benchmark rounds.
    ///
    /// A script render resets it on the way in; a benchmark that never renders
    /// would otherwise accumulate every round's nodes and measure the growth of
    /// the arena rather than the cost of writing to it.
    #[cfg(test)]
    pub(crate) fn reset_arena_for_benchmark(&self) {
        self.arena.borrow_mut().reset();
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

    fn purge_released_view_aliases(&self, release: &crate::entities::EntityRelease) {
        self.nested_view_handles
            .borrow_mut()
            .retain(|_, alias| !release.contains(alias.handle));
    }

    /// Resolves a release entirely under the store borrow, then performs all
    /// GPUI and callback/task retirement after the borrow has ended.
    pub(crate) fn release_view_handle(
        &self,
        handle: EntityHandle,
        cx: &mut impl gpui::AppContext,
    ) -> bool {
        let release = { self.entities().release_view(handle) };
        let Some(release) = release else {
            return false;
        };
        self.purge_released_view_aliases(&release);
        release.retire(cx);
        true
    }

    pub(crate) fn release_application_generation(
        &self,
        application: &Rc<ApplicationGeneration>,
        cx: &mut impl gpui::AppContext,
    ) {
        let release = { self.entities().release_application(application) };
        self.purge_released_view_aliases(&release);
        release.retire(cx);
        cancel_application_tasks(application);
    }

    pub(crate) fn release_application_generation_without_context(
        &self,
        application: &Rc<ApplicationGeneration>,
    ) {
        let release = { self.entities().release_application(application) };
        self.purge_released_view_aliases(&release);
        release.retire_without_context();
        cancel_application_tasks(application);
    }

    fn rollback_retained_since(
        &self,
        entities: crate::entities::EntityCheckpoint,
        tasks: scheduler::TaskCheckpoint,
        cx: &mut impl gpui::AppContext,
    ) {
        scheduler::rollback_runtime_tasks(tasks);
        let release = { self.entities().rollback(entities) };
        self.purge_released_view_aliases(&release);
        release.retire(cx);
    }

    #[cfg(test)]
    pub(crate) fn nested_view_alias_count(&self) -> usize {
        self.nested_view_handles.borrow().len()
    }

    fn job_queue_error(&self) -> Option<anyhow::Error> {
        self.terminal_job_error
            .borrow()
            .as_ref()
            .map(|message| anyhow!(message.clone()))
    }

    /// Permanently quarantines a runtime with an opaque unfinished job wave.
    fn fail_job_queue(&self) -> anyhow::Error {
        let message = "the QuickJS job queue exceeded gpui-shell's transactional limit; the \
                       script runtime was disabled so pending work cannot cross view authority"
            .to_owned();
        if self.terminal_job_error.borrow().is_none() {
            *self.terminal_job_error.borrow_mut() = Some(message.clone());
            scheduler::shutdown(self);
        }
        anyhow!(message)
    }

    /// Loads `main.js` from an application directory.
    ///
    /// Module resolution is scoped to that directory: an application can import
    /// its own files and the built-in modules, and nothing else. That is
    /// the first half of the sandbox's module policy (design doc §19.1).
    pub(crate) fn load_app(self: &Rc<Self>, dir: &Path, entry: &str) -> Result<ViewType> {
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
    #[cfg(test)]
    pub(crate) fn load_source(self: &Rc<Self>, name: &str, source: &str) -> Result<ViewType> {
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
    #[cfg(test)]
    pub(crate) fn instantiate(
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
                    self.release_application_generation(&application, cx);
                }
                return Err(error);
            }
        };
        if let Err(error) = self.initialize(&instance, None) {
            if let Some(application) = application {
                self.release_application_generation(&application, cx);
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
    #[cfg(test)]
    pub(crate) fn instantiate_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ScriptView>> {
        self.instantiate_view_with_policy(view_type, crate::policy::default(), window, cx)
    }

    pub(crate) fn instantiate_view_with_policy(
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
                    self.release_application_generation(&application, cx);
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
        let initialized = self.initialize(&object, None);
        let nested = self.flush_pending_nested_views(window, cx);
        if let Err(error) = initialized.and(nested) {
            if let Some(application) = application {
                self.release_application_generation(&application, cx);
            }
            return Err(error);
        }
        Ok(view)
    }

    /// Constructs, retains and initializes a nested view under its final GPUI
    /// entity owner.
    ///
    /// The handle enters the entity store before `init(props)` runs so anything
    /// init creates is tagged with the exact child owner. A failed init removes
    /// that handle and all records/tasks owned by the candidate child without
    /// touching application-wide state.
    pub(crate) fn instantiate_nested_view(
        self: &Rc<Self>,
        view_type: &ViewType,
        policy: Rc<crate::policy::Policy>,
        initial_props: Option<Persistent<Value<'static>>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<EntityHandle> {
        // Establish an empty queue boundary while the caller's scope is still
        // installed. Otherwise an older parent reaction would be executed by
        // the child-init drain and acquire the child's ownership/authority.
        scheduler::drain_jobs_transactionally(self, window, cx)?;
        let entity_checkpoint = { self.entities().checkpoint() };
        let task_checkpoint = scheduler::checkpoint_runtime_tasks(self);

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
        let constructed = self.construct(view_type);
        let construction_jobs = scheduler::drain_jobs_transactionally(self, window, cx);
        drop(construct_scope);
        let object = match construction_jobs.and(constructed) {
            Ok(object) => object,
            Err(error) => {
                self.rollback_retained_since(entity_checkpoint, task_checkpoint, cx);
                return Err(error);
            }
        };

        if self.entities().len() >= crate::entities::MAX_LIVE_ENTITIES {
            self.rollback_retained_since(entity_checkpoint, task_checkpoint, cx);
            anyhow::bail!(
                "the application reached gpui-shell's retained entity limit; release unused handles"
            );
        }
        let view =
            cx.new(|cx| ScriptView::nested(self.clone(), object, policy.clone(), cx.entity_id()));
        let handle = self
            .entities()
            .create_view(view.clone(), application.clone(), self);
        let object = view.read(cx).object().clone();

        let (_initialize_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application,
        );
        let initialized = self.initialize(&object, initial_props);
        let nested = self.flush_pending_nested_views(window, cx);
        // The queue was empty before init entered. Draining its whole causal
        // wave here therefore assigns only init continuations to the child and
        // prevents a throwing init from leaving work beyond local rollback.
        let init_jobs = scheduler::drain_jobs_transactionally(self, window, cx);
        if let Err(error) = initialized.and(nested).and(init_jobs) {
            scheduler::rollback_runtime_tasks(task_checkpoint);
            let released = self.release_view_handle(handle, cx);
            debug_assert!(released, "the candidate child handle must still be live");
            let residual = { self.entities().rollback(entity_checkpoint) };
            self.purge_released_view_aliases(&residual);
            residual.retire(cx);
            return Err(error);
        }
        Ok(handle)
    }

    /// Defers creation until the native host call has returned to Rust and the
    /// active `Context::with` lock has been released. The opaque token is all
    /// JavaScript keeps; it is resolved to the typed entity handle before the
    /// enclosing engine entry returns.
    fn queue_nested_view_creation(
        self: &Rc<Self>,
        ctx: &Ctx<'_>,
        class: Persistent<Object<'static>>,
        props: Persistent<Value<'static>>,
    ) -> JsResult<u32> {
        let parent = scope::current_view().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "cx.new(Class, props) needs a current script view; call it from a \
                 view's init(), event handler or task",
            )
        })?;
        let (parent_object, policy) = scope::with_current_app(|cx| {
            let parent = parent.read(cx);
            (parent.object().clone(), parent.policy())
        })
        .ok_or_else(|| nested_view_needs_call(ctx, "cx.new(Class, props)"))?;
        let provenance = self
            .initializing_views
            .borrow()
            .last()
            .cloned()
            .unwrap_or(parent_object);
        let application =
            scope::current_application_generation().or_else(|| provenance.application_generation());
        let token = self.next_nested_view_token.get();
        let next = token.checked_add(1).ok_or_else(|| {
            Exception::throw_range(ctx, "the nested Entity token space is exhausted")
        })?;
        self.next_nested_view_token.set(next);
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Create {
                runtime: Rc::downgrade(self),
                token,
                owner: parent,
                view_type: ViewType {
                    value: class,
                    module_lease: provenance.module_lease.clone(),
                    application,
                },
                policy,
                props,
            });
        Ok(token)
    }

    fn queue_nested_view_update(
        self: &Rc<Self>,
        ctx: &Ctx<'_>,
        token: u32,
        props: Persistent<Value<'static>>,
    ) -> JsResult<()> {
        let resolved = self.nested_view_handles.borrow().get(&token).cloned();
        let pending = self.pending_nested.borrow();
        let pending_create = pending.iter().find_map(|operation| match operation {
            PendingNestedOperation::Create {
                token: candidate,
                view_type,
                policy,
                ..
            } if *candidate == token => Some(NestedViewProvenance {
                application: view_type.application.clone(),
                policy: policy.clone(),
            }),
            _ => None,
        });
        let pending_release = pending.iter().any(|operation| {
            matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
        });
        drop(pending);
        let provenance = resolved
            .as_ref()
            .map(|alias| alias.provenance.clone())
            .or(pending_create)
            .or_else(|| self.in_flight_nested.borrow().get(&token).cloned());
        if pending_release || provenance.as_ref().is_none_or(|owner| !owner.is_current()) {
            return Err(Exception::throw_type(
                ctx,
                "this Entity has been released and can no longer be updated",
            ));
        }
        if resolved
            .as_ref()
            .is_some_and(|alias| self.entities().view(alias.handle).is_none())
        {
            self.nested_view_handles.borrow_mut().remove(&token);
            return Err(Exception::throw_type(
                ctx,
                "this Entity has been released and can no longer be updated",
            ));
        }
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Update {
                runtime: Rc::downgrade(self),
                token,
                provenance: provenance.expect("validated nested provenance"),
                props,
            });
        Ok(())
    }

    fn queue_nested_view_release(self: &Rc<Self>, ctx: &Ctx<'_>, token: u32) -> JsResult<bool> {
        let pending = self.pending_nested.borrow();
        let pending_release = pending.iter().any(|operation| {
            matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
        });
        let pending_create = pending.iter().find_map(|operation| match operation {
            PendingNestedOperation::Create {
                token: candidate,
                view_type,
                policy,
                ..
            } if *candidate == token => Some(NestedViewProvenance {
                application: view_type.application.clone(),
                policy: policy.clone(),
            }),
            _ => None,
        });
        drop(pending);
        let resolved = self.nested_view_handles.borrow().get(&token).cloned();
        let provenance = resolved
            .as_ref()
            .map(|alias| alias.provenance.clone())
            .or(pending_create)
            .or_else(|| self.in_flight_nested.borrow().get(&token).cloned());
        if provenance.as_ref().is_none_or(|owner| !owner.is_current()) {
            return Err(Exception::throw_type(
                ctx,
                "this Entity has been released and can no longer be released",
            ));
        }
        // Authority is checked before resolving entity liveness or changing
        // the alias table. A foreign caller must not distinguish a live token
        // from a dead one, nor clean up an alias owned by another application.
        if resolved
            .as_ref()
            .is_some_and(|alias| self.entities().view(alias.handle).is_none())
        {
            self.nested_view_handles.borrow_mut().remove(&token);
            return Ok(false);
        }
        if pending_release {
            return Ok(false);
        }
        self.pending_nested
            .borrow_mut()
            .push_back(PendingNestedOperation::Release {
                runtime: Rc::downgrade(self),
                token,
                provenance: provenance.expect("validated nested provenance"),
            });
        Ok(true)
    }

    /// Applies native nested-view requests only at an unlocked QuickJS
    /// boundary. Construction therefore goes through Task 2's exact
    /// `instantiate_nested_view` seam and its three bounded causal drains.
    pub(super) fn flush_pending_nested_views(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        if self.flushing_nested.replace(true) {
            return Ok(());
        }
        let _flush_guard = NestedFlushGuard(&self.flushing_nested);
        loop {
            let operation = { self.pending_nested.borrow_mut().pop_front() };
            let Some(operation) = operation else {
                break;
            };
            let result = (|| -> Result<()> {
                match operation {
                    PendingNestedOperation::Create {
                        runtime,
                        token,
                        owner,
                        view_type,
                        policy,
                        props,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child creation")
                        })?;
                        let provenance = NestedViewProvenance {
                            application: view_type.application.clone(),
                            policy: policy.clone(),
                        };
                        if !provenance.is_current() {
                            anyhow::bail!(
                                "this Entity creation does not belong to the current application"
                            );
                        }
                        runtime
                            .in_flight_nested
                            .borrow_mut()
                            .insert(token, provenance.clone());
                        let (_owner_scope, _) = scope::enter_with_application(
                            &runtime,
                            window,
                            cx,
                            ScopePhase::Event,
                            Some(owner),
                            policy.clone(),
                            view_type.application.clone(),
                        );
                        let handle = runtime.instantiate_nested_view(
                            &view_type,
                            policy,
                            Some(props),
                            window,
                            cx,
                        );
                        runtime.in_flight_nested.borrow_mut().remove(&token);
                        let handle = handle?;
                        runtime
                            .nested_view_handles
                            .borrow_mut()
                            .insert(token, NestedViewAlias { handle, provenance });
                        Ok(())
                    }
                    PendingNestedOperation::Update {
                        runtime,
                        token,
                        provenance,
                        props,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child update")
                        })?;
                        if !provenance.is_current() {
                            anyhow::bail!("this Entity does not belong to the current application");
                        }
                        let handle = runtime
                            .nested_view_handles
                            .borrow()
                            .get(&token)
                            .filter(|alias| {
                                Rc::ptr_eq(&alias.provenance.policy, &provenance.policy)
                                    && match (
                                        &alias.provenance.application,
                                        &provenance.application,
                                    ) {
                                        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                                        (None, None) => true,
                                        _ => false,
                                    }
                            })
                            .map(|alias| alias.handle)
                            .ok_or_else(|| anyhow!("this Entity was released before its update"))?;
                        runtime.update_nested_view(handle, props, window, cx)?;
                        Ok(())
                    }
                    PendingNestedOperation::Release {
                        runtime,
                        token,
                        provenance,
                    } => {
                        let runtime = runtime.upgrade().ok_or_else(|| {
                            anyhow!("the shell runtime shut down during child release")
                        })?;
                        if !provenance.is_current() {
                            anyhow::bail!("this Entity does not belong to the current application");
                        }
                        let alias = runtime
                            .nested_view_handles
                            .borrow()
                            .get(&token)
                            .cloned()
                            .filter(|alias| {
                                Rc::ptr_eq(&alias.provenance.policy, &provenance.policy)
                                    && match (
                                        &alias.provenance.application,
                                        &provenance.application,
                                    ) {
                                        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                                        (None, None) => true,
                                        _ => false,
                                    }
                            })
                            .ok_or_else(|| {
                                anyhow!("this Entity was released before its release operation")
                            })?;
                        runtime.nested_view_handles.borrow_mut().remove(&token);
                        let handle = alias.handle;
                        let released = runtime.release_view_handle(handle, cx);
                        if !released {
                            anyhow::bail!("this Entity was released before its release operation");
                        }
                        Ok(())
                    }
                }
            })();
            if let Err(error) = result {
                self.pending_nested.borrow_mut().clear();
                return Err(error);
            }
        }
        Ok(())
    }

    /// Delivers props to the child under a bounded ordinary-state/resource
    /// rollback boundary, and refreshes only after the causal wave succeeds.
    /// Ordinary properties on reachable objects and callable objects are
    /// restorable only while their post-update descriptors remain legally
    /// redefinable/deletable. Private/internal JS state, non-configurable
    /// additions/hardening and destructive release of pre-existing native
    /// handles are outside that boundary.
    fn update_nested_view(
        self: &Rc<Self>,
        handle: EntityHandle,
        props: Persistent<Value<'static>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        scheduler::drain_jobs_transactionally(self, window, cx)?;
        // End the store borrow before update or any continuation can re-enter
        // JavaScript.
        let view = { self.entities().view(handle) }
            .ok_or_else(|| anyhow!("this Entity has been released and cannot be updated"))?;
        let (object, policy, application) = {
            let child = view.read(cx);
            (
                child.object().clone(),
                child.policy(),
                child.application_generation(),
            )
        };
        let state_checkpoint = self.checkpoint_view_object(&object)?;
        let entity_checkpoint = { self.entities().checkpoint() };
        let task_checkpoint = scheduler::checkpoint_runtime_tasks(self);
        let (event_scope, _) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            Some(view.clone()),
            policy,
            application,
        );
        let updated = self.with_js(|ctx| {
            let props = props.restore(ctx)?;
            self.update_in_context(ctx, &object, props)
        });
        let update_jobs = scheduler::drain_jobs_transactionally(self, window, cx);
        drop(event_scope);
        match update_jobs.and(updated) {
            Ok(()) => {
                view.update(cx, |view, cx| view.refresh(cx));
                Ok(())
            }
            Err(error) => {
                let restored = self.restore_view_object(state_checkpoint);
                self.rollback_retained_since(entity_checkpoint, task_checkpoint, cx);
                if let Err(restore) = restored {
                    return Err(error.context(format!("failed to restore child state: {restore}")));
                }
                Err(error)
            }
        }
    }

    pub(crate) fn instantiate_for_view(
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
                    self.release_application_generation(&application, cx);
                }
                return Err(error);
            }
        };
        let initialized = self.initialize(&object, None);
        let nested = self.flush_pending_nested_views(window, cx);
        if let Err(error) = initialized.and(nested) {
            if let Some(application) = application {
                self.release_application_generation(&application, cx);
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

    /// Captures reachable ordinary objects and callable objects without invoking getters.
    /// The returned closure restores descriptors in place only when their
    /// post-update state still permits the required redefinition/deletion,
    /// preserving object identity for callbacks and tasks that already captured
    /// the instance. Private/internal slots, non-configurable additions, and a
    /// property hardened from configurable to non-configurable cannot be
    /// restored by JavaScript reflection.
    fn checkpoint_view_object(&self, object: &ViewObject) -> Result<ViewStateCheckpoint> {
        self.with_js(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let checkpoint: Function = ctx.globals().get("__checkpoint_view")?;
            let restore: Function = checkpoint.call((instance,))?;
            Ok(ViewStateCheckpoint(Persistent::save(ctx, restore)))
        })
    }

    fn restore_view_object(&self, checkpoint: ViewStateCheckpoint) -> Result<()> {
        self.with_js(|ctx| {
            let restore = checkpoint.0.restore(ctx)?;
            restore.call::<_, ()>(())
        })
    }

    fn initialize(
        &self,
        object: &ViewObject,
        initial_props: Option<Persistent<Value<'static>>>,
    ) -> Result<()> {
        self.initializing_views.borrow_mut().push(object.clone());
        let initialized = self.with_js(|ctx| {
            let instance = object.value.clone().restore(ctx)?;
            let initialize: Function = ctx.globals().get("__initialize")?;
            let props = match initial_props {
                Some(props) => props.restore(ctx)?,
                None => Value::new_undefined(ctx.clone()),
            };
            initialize.call::<_, ()>((instance, props))
        });
        let initializing = self.initializing_views.borrow_mut().pop();
        debug_assert!(initializing.is_some());
        initialized
    }

    fn update_in_context<'js>(
        &self,
        ctx: &Ctx<'js>,
        object: &ViewObject,
        props: Value<'js>,
    ) -> JsResult<()> {
        let instance = object.value.clone().restore(ctx)?;
        let update: Value = instance.get("update")?;
        if update.is_undefined() || update.is_null() {
            return Ok(());
        }
        let update = update.as_function().ok_or_else(|| {
            Exception::throw_type(ctx, "a nested view's update property must be a function")
        })?;
        update.call::<_, ()>((This(instance), props))
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
    pub(crate) fn render_to_spec(
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

    /// Retires every callback registered by one retained view, including
    /// generations still held by a rendered frame.
    pub(crate) fn retire_view_callbacks(&self, entity_id: gpui::EntityId) {
        self.callbacks.borrow_mut().retain(|entry| {
            entry
                .view
                .as_ref()
                .is_none_or(|owner| owner.entity_id() != entity_id)
        });
    }

    /// How many handlers are callable right now. See [`CallbackArena::len`].
    #[cfg(test)]
    pub(crate) fn live_callbacks(&self) -> usize {
        self.callbacks.borrow().len()
    }

    #[cfg(test)]
    pub(crate) fn live_callback_ids(&self) -> Vec<CallbackId> {
        self.callbacks.borrow().ids()
    }

    /// Describes one window of a virtualized list's items.
    ///
    /// The one call into script that is *not* a snapshot build and *not* an
    /// event: GPUI runs it from inside layout and prepaint, so it happens on a
    /// frame's budget rather than on an application's. See the exception
    /// recorded in [`crate::materialize`] for why that trade is the right one
    /// here and nowhere else.
    ///
    /// Three things make it safe to enter the VM from there:
    ///
    /// * The scope is [`ScopePhase::Layout`], which forbids `cx.notify()` —
    ///   a re-render requested from inside layout is a loop — along with
    ///   creating retained state, and runs on the render-time budget.
    /// * The batch describes itself into an arena of its own, swapped in for
    ///   the duration. The runtime's scratch arena belongs to whichever script
    ///   render is in progress; a batch writing into it would survive into that
    ///   render's snapshot. Swapping is strictly nested, so a list inside a
    ///   list is no different from one on its own.
    /// * Nothing is drained afterwards. `dispatch_click` runs QuickJS's job
    ///   queue on the way out because an event handler may have resolved a
    ///   promise; a continuation is application code of unbounded length, and
    ///   running one part-way through GPUI's layout pass is the last place it
    ///   belongs. Queued jobs wait for the event loop, as they would have
    ///   anyway.
    pub(crate) fn render_virtual_items(
        self: &Rc<Self>,
        id: CallbackId,
        get_key: CallbackId,
        range: std::ops::Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<crate::spec::ItemSpecs> {
        let entry = self.callbacks.borrow().get(id)?;
        let key_entry = self.callbacks.borrow().get(get_key)?;

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("item renderer {id} belongs to a retired application");
            return None;
        }

        let view = entry.live_view()?;
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Layout,
            view,
            policy,
            entry.application.clone(),
        );

        let outer = std::mem::take(&mut *self.arena.borrow_mut());
        let described = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let key_handler = key_entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            payload.set("start", range.start)?;
            payload.set("end", range.end)?;
            let produced: Value =
                handler.call((payload, context_object(ctx, ContextBinding::Call(generation))?))?;
            let items = produced.into_array().ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "a virtual list's item renderer must return an array of elements, one per                      item in the range it was given",
                )
            })?;
            let mut roots = SmallVec::new();
            for item in items.iter::<Value>() {
                roots.push(element_id(ctx, &item?)?);
            }
            let mut keys = Vec::new();
            keys.try_reserve_exact(range.len()).map_err(|_| {
                Exception::throw_range(ctx, "the virtual list item-key table could not be allocated")
            })?;
            let mut unique = HashSet::new();
            for index in range.clone() {
                let key: String = key_handler.call((index,))?;
                if !unique.insert(key.clone()) {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!("virtual list get_key returned duplicate key `{key}` in one visible range"),
                    ));
                }
                keys.push(key);
            }
            Ok((roots, keys))
        });
        let arena = std::mem::replace(&mut *self.arena.borrow_mut(), outer);

        match described {
            Ok((roots, keys)) => Some(crate::spec::ItemSpecs::new(arena, roots, keys)),
            Err(error) => {
                tracing::error!("error in virtual list item renderer: {error}");
                None
            }
        }
    }

    /// Reports which stable item of a collection something happened to.
    pub(crate) fn dispatch_item_key(
        self: &Rc<Self>,
        id: CallbackId,
        key: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("item callback {id} belongs to a superseded render pass");
            return;
        };

        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("item callback {id} belongs to a retired application");
            return;
        }

        let Some(view) = entry.live_view() else {
            tracing::debug!("item callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((key, context_object(ctx, ContextBinding::Call(generation))?))
        });

        if let Err(error) = result {
            tracing::error!("error in item click handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    /// Delivers a one-time-code event to a long-lived script subscription.
    pub(super) fn dispatch_otp_event(
        self: &Rc<Self>,
        handler: &Persistent<Function<'static>>,
        owner: &InputCallbackOwner,
        _event: &gpui_base::OtpEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if owner
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("OTP callback belongs to a retired application");
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
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in OTP handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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

        let Some(view) = entry.live_view() else {
            tracing::debug!("click callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
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

            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in click handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in input handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in slider handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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

        let Some(view) = entry.live_view() else {
            tracing::debug!("resize callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = rquickjs::Array::new(ctx.clone())?;
            for (index, size) in sizes.iter().enumerate() {
                payload.set(index, *size)?;
            }
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in resize handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
    }

    pub(crate) fn dispatch_mouse_move(
        self: &Rc<Self>,
        id: CallbackId,
        event: &gpui::MouseMoveEvent,
        local: gpui::Point<gpui::Pixels>,
        bounds: gpui::Bounds<gpui::Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entry) = self.callbacks.borrow().get(id) else {
            tracing::debug!("mouse move callback {id} belongs to a superseded render pass");
            return;
        };
        if entry
            .application
            .as_ref()
            .is_some_and(|application| !application.is_active())
        {
            tracing::debug!("mouse move callback {id} belongs to a retired application");
            return;
        }
        let Some(view) = entry.live_view() else {
            tracing::debug!("mouse move callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );
        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            let payload = Object::new(ctx.clone())?;
            let position = Object::new(ctx.clone())?;
            position.set("x", f32::from(event.position.x))?;
            position.set("y", f32::from(event.position.y))?;
            payload.set("position", position)?;
            let local_position = Object::new(ctx.clone())?;
            local_position.set("x", f32::from(local.x))?;
            local_position.set("y", f32::from(local.y))?;
            payload.set("local_position", local_position)?;
            let event_bounds = Object::new(ctx.clone())?;
            event_bounds.set("x", f32::from(bounds.origin.x))?;
            event_bounds.set("y", f32::from(bounds.origin.y))?;
            event_bounds.set("width", f32::from(bounds.size.width))?;
            event_bounds.set("height", f32::from(bounds.size.height))?;
            payload.set("bounds", event_bounds)?;
            let modifiers = Object::new(ctx.clone())?;
            modifiers.set("shift", event.modifiers.shift)?;
            modifiers.set("control", event.modifiers.control)?;
            modifiers.set("alt", event.modifiers.alt)?;
            modifiers.set("platform", event.modifiers.platform)?;
            payload.set("modifiers", modifiers)?;
            handler.call::<_, ()>((
                payload,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });
        if let Err(error) = result {
            tracing::error!("error in mouse move handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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

        let Some(view) = entry.live_view() else {
            tracing::debug!("change callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                checked,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in change handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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

        let Some(view) = entry.live_view() else {
            tracing::debug!("step callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                action,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in step handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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

        let Some(view) = entry.live_view() else {
            tracing::debug!("signal callback {id} owner has been released");
            return;
        };
        let policy = view
            .as_ref()
            .map(|view| view.read(cx).policy())
            .unwrap_or_else(crate::policy::default);
        let (_guard, generation) = scope::enter_with_application(
            self,
            window,
            cx,
            ScopePhase::Event,
            view,
            policy,
            entry.application.clone(),
        );

        let result = self.with_js(|ctx| {
            let handler = entry.value.clone().restore(ctx)?;
            handler.call::<_, ()>((
                Object::new(ctx.clone())?,
                context_object(ctx, ContextBinding::Call(generation))?,
            ))
        });

        if let Err(error) = result {
            tracing::error!("error in signal handler: {error}");
        }
        scheduler::drain_runtime_jobs(self, window, cx);
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
            let produced: Value = render.call((
                This(instance),
                context_object(ctx, ContextBinding::Call(generation))?,
            ))?;
            element_id(ctx, &produced)
        })
    }

    /// Runs `body` inside the JS context, flattening any exception into an
    /// ordinary error carrying the script's message and stack.
    fn with_js<T>(&self, body: impl FnOnce(&Ctx<'_>) -> JsResult<T>) -> Result<T> {
        if let Some(error) = self.job_queue_error() {
            return Err(error);
        }
        let pending_checkpoint = self.pending_nested.borrow().len();
        sandbox::begin_host_execution();
        let result = self.context.with(|ctx| match body(&ctx) {
            Ok(value) => Ok(value),
            Err(error) => Err(anyhow!("{}", describe(&ctx, error))),
        });
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.pending_nested
                    .borrow_mut()
                    .truncate(pending_checkpoint);
                Err(error)
            }
        }
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
            // An `OtpInput`'s three: one template for every cell, one layered
            // on the cell taking the next digit, and one for the caret in it.
            "cell_style" => "cell_style",
            "cell_active_style" => "cell_active_style",
            "caret_style" => "caret_style",
            other => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!(
                        "unknown state style `{other}`; expected hover, active, focus, \
                         range_style, cell_style, cell_active_style or caret_style"
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
            // A bare specifier reached the last resolver in the chain, so it is
            // neither a built-in nor a file. Saying which built-ins this
            // runtime does have is the difference between "you typed it wrong"
            // and "this binary is older than the script it is loading" — and
            // the second is what a moved module looks like from here.
            if !name.starts_with('.') && !name.contains('/') {
                return Err(Exception::throw_message(
                    ctx,
                    &format!(
                        "cannot resolve module `{name}`: this runtime's built-in modules are {}, \
                         and an application may otherwise import only its own files. If the \
                         script expects a module this runtime does not have, the two are \
                         different versions.",
                        builtin_specifiers()
                    ),
                ));
            }
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

  // Styles do not go through `__apply`, and the reason is arithmetic. They are
  // most of what a description records, and the generic form above pays three
  // times over for information the prelude already has: it allocates a rest
  // array to hold arguments a style never has more than one of, it sends a
  // method name that has to be copied into a Rust string, and it arrives at a
  // dispatcher that has to look that string back up in a table. Closing over
  // the table index instead removes all three, and measured at roughly half
  // the cost of a recorded style call. `define` stays for the behaviours,
  // where the argument shapes vary and a second form would not repay itself.
  const defineNullaryStyle = (name, index) => {
    methods[name] = function () {
      __applyNullaryStyle(this.__id, index);
      return this;
    };
  };
  const defineParamStyle = (name, index) => {
    methods[name] = function (value) {
      __applyParamStyle(this.__id, index, value);
      return this;
    };
  };

  for (let i = 0; i < __nullaryStyles.length; i += 1) {
    defineNullaryStyle(__nullaryStyles[i], __nullaryStyleIndexes[i]);
  }
  for (let i = 0; i < __paramStyles.length; i += 1) {
    defineParamStyle(__paramStyles[i], i);
  }
  for (const name of __behaviorNames) define(name);

  // Attaching is the other call a description makes once per element, and it
  // carries no argument a `Bridged` could describe — two element ids, both
  // already numbers. It gets an entry point of its own for the same reason the
  // styles do.
  //
  // A retained child view is a child too — the same shape GPUI has, where an
  // `Entity<V>` is itself renderable — so `.child(handle)` mounts one. An
  // element always carries `__id`, so the branch costs the hot path one
  // `undefined` test and the slow side is the case that used to fail with
  // rquickjs' "Error converting from undefined to f64".
  const childId = (child) => {
    const id = child?.__id;
    if (id !== undefined) return id;
    // A string is an element. GPUI implements `IntoElement` for `&str`,
    // `String` and `SharedString`, so `.child("hello")` is how text is written
    // there, and the style comes from the element holding it.
    const kind = typeof child;
    if (kind === "string" || kind === "number" || kind === "boolean") {
      return __text(String(child));
    }
    if (child?.__entity) return __child_view(child.__handle);
    throw new TypeError(
      "child(value) expects an element, a string, or an entity from cx.new(Class, props)",
    );
  };
  methods.child = function (child) {
    __attach(this.__id, childId(child));
    return this;
  };
  methods.children = function (list) {
    for (const child of list) __attach(this.__id, childId(child));
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
        "track_focus(handle) expects a FocusHandle from cx.focus_handle(), not a name or an element",
      );
    }
    __apply(this.__id, "track_focus", [handle.__handle]);
    return this;
  };
  // A virtualized list's scroll position, unwrapped exactly as `track_focus`
  // unwraps a focus handle, and checked here for the same reason: a name or an
  // element would be dropped on the Rust side and the list would simply never
  // respond to `scroll_to_item`.
  methods.track_scroll = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "track_scroll(handle) expects a VirtualListScrollHandle from VirtualListScrollHandle.new()",
      );
    }
    __apply(this.__id, "track_scroll", [handle.__handle]);
    return this;
  };
  // The second handle a combobox root needs: the one the keyboard moves to when
  // the surface opens. Checked here for the same reason `track_focus` is — a
  // name or an element would otherwise be dropped on the Rust side and the
  // focus would simply never move.
  methods.content_focus_handle = function (handle) {
    if (typeof handle?.__handle !== "number") {
      throw new TypeError(
        "content_focus_handle(handle) expects a FocusHandle from cx.focus_handle(), not a name or an element",
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
  // An OtpInput's cells. Not states either: the shell decides which template
  // a cell gets, from the state, on every frame — but they are declared the
  // same way, because what they collect is the same thing.
  methods.cell_style = state("cell_style");
  methods.cell_active_style = state("cell_active_style");
  methods.caret_style = state("caret_style");

  // The argument checks are here rather than only on the Rust side because a
  // list built with the pieces in the wrong order — a render function where the
  // sizes go — would otherwise fail as a type error naming neither.
  const virtualList = (build, name) => (id, item_count, item_sizes, get_key, render) => {
    const shape = name + "(id, item_count, item_sizes, get_key, render)";
    if (!Number.isInteger(item_count) || item_count < 0) {
      throw new TypeError(shape + " needs a whole, non-negative item_count");
    }
    if (typeof render !== "function") {
      throw new TypeError(
        shape + " needs a render function; it is called once per visible range, not once per item",
      );
    }
    if (typeof get_key !== "function") {
      throw new TypeError(
        shape + " needs get_key(index) to return each item's stable string key",
      );
    }
    if (Array.isArray(item_sizes) && item_sizes.length !== item_count) {
      throw new TypeError(
        shape + " was given " + item_sizes.length + " item sizes for " + item_count +
          " items; pass one number for a uniform extent, or one per item",
      );
    }
    return element(build(String(id), item_count, item_sizes, get_key, render));
  };

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
    const duration = finiteDuration(policy.duration ?? 0, "transition duration");
    const delay = finiteDuration(policy.delay ?? 0, "transition delay");
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
    const response = finiteDuration(policy.response ?? 250, "spring response");
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
  const finiteDuration = (value, name) => {
    value = finiteNonNegative(value, name);
    if (value > 86400000) throw new RangeError(name + " must not exceed 86400000 milliseconds");
    return value;
  };
  const delay = (name) =>
    function (ms) {
      __apply(this.__id, name, [finiteDuration(ms, name)]);
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
      if (!Array.isArray(values) || values.some((value) => typeof value !== "number" || !Number.isFinite(value) || Math.fround(value) <= 0)) {
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
    if (!pathValue?.__path) throw new TypeError("window.paint_path(path, background) expects a Path built by PathBuilder");
    const paint = asBackground(paintValue);
    const object = element(__path(
      pathValue.fill,
      paint.kind,
      paint.values.map(String),
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

  // A one-time code. `len` is read rather than set: base fixes it when the
  // state is created and offers no setter, because it is what the state is.
  const otpState = (handle) => ({
    __handle: handle,
    value: () => __otp_value(handle),
    set_value: (next) => __otp_set_value(handle, String(next ?? "")),
    len: () => __otp_len(handle),
    is_masked: () => __otp_is_masked(handle),
    set_masked: (masked) => __otp_set_masked(handle, Boolean(masked)),
    focus: () => __otp_focus(handle),
    on: (event, handler) => __otp_on(handle, String(event), handler),
    release: () => __otp_release(handle),
  });

  const focusHandle = (handle) => ({
    __handle: handle,
    focus: () => __focus_focus(handle),
    is_focused: () => __focus_is_focused(handle),
    release: () => __focus_release(handle),
  });

  // `__entity` is the discriminant `.child()` needs: every retained handle in
  // this API is a `{__handle: number}` wrapper, so a focus handle and an entity
  // are otherwise indistinguishable, and mounting the wrong one would report a
  // released view rather than the mistake that was made.
  const entity = (handle) => ({
    __entity: true,
    __handle: handle,
    set_props: (props) => __view_set_props(handle, props),
    release: () => __view_release(handle),
  });

  const virtualScrollHandle = (handle) => ({
    __handle: handle,
    // The strategy is base's own word for where the item lands. `top` puts it
    // at the near edge, `center` in the middle; base's default is `top`.
    scroll_to_item: (index, strategy) => {
      if (!Number.isInteger(index) || index < 0) {
        throw new TypeError("scroll_to_item(index) needs a whole, non-negative index");
      }
      __virtual_scroll_to_item(handle, index, String(strategy ?? "top"));
    },
    scroll_to_bottom: () => __virtual_scroll_to_bottom(handle),
    release: () => __virtual_scroll_release(handle),
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
  // `init` gets the async flavor, and both of its paths get the same one. That
  // is the honest shape: `init` exists to set up things that outlive the call —
  // tasks, timers, retained handles — so the context it hands to them must
  // outlive it too. A call-scoped `cx` here could not be given to the very work
  // `init` is for.
  globalThis.__initialize = (instance, props) => {
    if (typeof instance.init === "function") instance.init(props, __async_cx());
  };
  // This journals ordinary reachable object and callable descriptors only. Restoration succeeds
  // only while post-update descriptors remain legally redefinable/deletable.
  // Reflection cannot see private/internal slots or undo non-configurable
  // additions/hardening; the public declaration documents that boundary.
  globalThis.__checkpoint_view = (instance) => {
    const snapshots = [];
    const seen = new Set();
    const pending = [instance];
    let propertyCount = 0;
    while (pending.length > 0) {
      const value = pending.pop();
      if (
        value === null ||
        (typeof value !== "object" && typeof value !== "function") ||
        seen.has(value)
      ) continue;
      if (snapshots.length >= 10_000) {
        throw new RangeError("a nested view update reached the 10,000-object rollback limit");
      }
      seen.add(value);
      const descriptors = Object.getOwnPropertyDescriptors(value);
      const keys = Reflect.ownKeys(descriptors);
      propertyCount += keys.length;
      if (propertyCount > 100_000) {
        throw new RangeError("a nested view update reached the 100,000-property rollback limit");
      }
      snapshots.push([value, descriptors]);
      for (const key of keys) {
        const descriptor = descriptors[key];
        if (Object.prototype.hasOwnProperty.call(descriptor, "value")) {
          pending.push(descriptor.value);
        }
      }
    }
    return () => {
      for (let index = snapshots.length - 1; index >= 0; index -= 1) {
        const [value, descriptors] = snapshots[index];
        const saved = new Set(Reflect.ownKeys(descriptors));
        for (const key of Reflect.ownKeys(value)) {
          if (!saved.has(key)) {
            const current = Object.getOwnPropertyDescriptor(value, key);
            if (current?.configurable) delete value[key];
          }
        }
        Object.defineProperties(value, descriptors);
      }
    };
  };

  class View {
    constructor(props) {
      // `new MyView(props)` from script reaches `init` without the host's
      // generation, so the context here is the async flavor — it resolves
      // whichever call is running, and says so if there is none.
      if (!deferInit && typeof this.init === "function") this.init(props, __async_cx());
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

    // `Window::paint_path` in GPUI, so `window` here. It is the one element
    // constructor that is not a free function, and it is one because the thing
    // it mirrors is a method on the window rather than on the app.
    paint_path: paintPath,
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

  // Every member of `cx` gates on `check()` and then does ordinary ambient
  // work. That gate is the whole difference between a call-scoped `cx` and an
  // async one, so a member added here is right for both flavors at once and
  // the two cannot drift.
  const contextMembers = (check) => ({
    theme: () => {
      check();
      return currentTheme();
    },
    open_url: (url) => {
      check();
      return __open_url(String(url));
    },
    read_from_clipboard: () => {
      check();
      return __clipboard_read_text();
    },
    write_to_clipboard: (text) => {
      check();
      return __clipboard_write_text(String(text));
    },
    focus_handle: () => {
      check();
      return focusHandle(__focus_handle_new());
    },
    new: (Class, props) => {
      check();
      if (typeof Class !== "function" || !(Class.prototype instanceof View)) {
        throw new TypeError("cx.new(Class, props) expects a View subclass");
      }
      return entity(__view_new(Class, props));
    },
    spawn: (body, opts) => {
      check();
      return __spawn(body, opts);
    },
    sleep: (ms) => {
      check();
      return __sleep(ms);
    },
    timer: {
      after: (ms, handler, opts) => {
        check();
        return __timer_after(ms, handler, opts);
      },
      every: (ms, handler, opts) => {
        check();
        return __timer_every(ms, handler, opts);
      },
    },
  });

  return {
    View,
    div: () => element(__div()),
    h_flex: () => element(__h_flex()),
    v_flex: () => element(__v_flex()),
    svg: (path) => element(__svg(String(path))),
    image: (path) => element(__image(String(path))),
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
    __context_members: contextMembers,
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
            "DatePicker.new(id, focus_handle) expects a FocusHandle from cx.focus_handle(); the picker takes the keyboard through that handle, and base has no builder to supply one later",
          );
        }
        return element(__date_picker(String(id), focus_handle.__handle));
      },
    },
    // Free functions, not `VirtualList.new(...)`, because that is what base
    // exports: `v_virtual_list` and `h_virtual_list` are the whole of its
    // public surface, and the list has no type a script ever names.
    //
    // The count is a separate argument from the sizes, which base does not
    // separate — its one vector is both. See the `.d.ts` for why: mirroring it
    // would put one number per row across the boundary on every render.
    v_virtual_list: virtualList(__v_virtual_list, "v_virtual_list"),
    h_virtual_list: virtualList(__h_virtual_list, "h_virtual_list"),
    VirtualListScrollHandle: { new: () => virtualScrollHandle(__virtual_scroll_new()) },
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
    OtpState: {
      new: (length, options) => {
        // A code of no cells accepts no keystroke and shows nothing, and a
        // typed-in length of six hundred thousand is a frozen window. Neither
        // is something base refuses, so it is refused here.
        if (!Number.isInteger(length) || length < 1 || length > 64) {
          throw new TypeError("OtpState.new(length) expects a whole number between 1 and 64");
        }
        const settings = options ?? {};
        return otpState(
          __otp_state_new(length, settings.value ?? null, Boolean(settings.masked)),
        );
      },
    },
    OtpInput: { new: (state) => element(__otp_element(state.__handle)) },
  };
})();
"#;

impl ShellRuntime {
    fn install_globals(self: &Rc<Self>) -> Result<()> {
        let runtime = Rc::downgrade(self);
        self.with_js(move |ctx| {
            let globals = ctx.globals();

            // Two tables rather than one list of names: the prelude binds a
            // different prototype method over each, and both close over the
            // index that identifies the style, so that recording one never
            // puts its name on the wire.
            let nullary = rquickjs::Array::new(ctx.clone())?;
            let nullary_indexes = rquickjs::Array::new(ctx.clone())?;
            for (position, (name, index)) in style::nullary_styles().into_iter().enumerate() {
                nullary.set(position, name)?;
                nullary_indexes.set(position, index)?;
            }
            globals.set("__nullaryStyles", nullary)?;
            globals.set("__nullaryStyleIndexes", nullary_indexes)?;

            let parametric = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in style::param_styles().enumerate() {
                parametric.set(index, name)?;
            }
            globals.set("__paramStyles", parametric)?;

            let behaviors = rquickjs::Array::new(ctx.clone())?;
            for (index, name) in [
                "on_click",
                "on_mouse_move",
                "on_hover",
                "on_item_click",
                "on_change",
                "on_open_change",
                "on_confirm",
                "on_dismiss",
                "on_step",
                "disabled",
                "selected",
                "checked",
                "accessibility_label",
                "tooltip",
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
                "with_item_to_measure_index",
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
                          values: Array<'_>,
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
                        let value_count =
                            crate::engine::quickjs::native::bridge_array_len(&ctx, &values)?;
                        let mut value_strings = Vec::new();
                        value_strings.try_reserve_exact(value_count).map_err(|_| {
                            Exception::throw_range(&ctx, "path background values are too large")
                        })?;
                        for index in 0..value_count {
                            value_strings.push(values.get::<String>(index)?);
                        }
                        let number = |index: usize, name: &str| -> JsResult<f32> {
                            value_strings
                                .get(index)
                                .and_then(|value| value.parse::<f32>().ok())
                                .filter(|value| value.is_finite())
                                .ok_or_else(|| Exception::throw_type(&ctx, name))
                        };
                        let text = |index: usize, name: &str| -> JsResult<String> {
                            value_strings
                                .get(index)
                                .cloned()
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
            virtual_list_constructor(
                &globals,
                "__v_virtual_list",
                runtime.clone(),
                gpui::Axis::Vertical,
            )?;
            virtual_list_constructor(
                &globals,
                "__h_virtual_list",
                runtime.clone(),
                gpui::Axis::Horizontal,
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

            let create_view = runtime.clone();
            globals.set(
                "__view_new",
                Func::from(
                    move |ctx: Ctx<'_>, class: NestedViewClass, props: NestedViewProps| {
                        refuse_nested_view_mutation(
                            &ctx,
                            "cx.new(Class, props)",
                            "create",
                        )?;
                        let runtime = upgrade(&create_view, &ctx)?;
                        runtime.queue_nested_view_creation(&ctx, class.0, props.0)
                    },
                ),
            )?;

            let update_view = runtime.clone();
            globals.set(
                "__view_set_props",
                Func::from(move |ctx: Ctx<'_>, token: u32, props: NestedViewProps| {
                    refuse_nested_view_mutation(&ctx, "entity.set_props(props)", "update")?;
                    let runtime = upgrade(&update_view, &ctx)?;
                    runtime.queue_nested_view_update(&ctx, token, props.0)
                }),
            )?;

            let release_view = runtime.clone();
            globals.set(
                "__view_release",
                Func::from(move |ctx: Ctx<'_>, token: u32| -> JsResult<bool> {
                    refuse_nested_view_mutation(&ctx, "entity.release()", "release")?;
                    let runtime = upgrade(&release_view, &ctx)?;
                    runtime.queue_nested_view_release(&ctx, token)
                }),
            )?;

            let mount_view = runtime.clone();
            globals.set(
                "__child_view",
                Func::from(move |ctx: Ctx<'_>, token: u32| -> JsResult<SpecId> {
                    let runtime = upgrade(&mount_view, &ctx)?;
                    if runtime.pending_nested.borrow().iter().any(|operation| {
                        matches!(operation, PendingNestedOperation::Release { token: candidate, .. } if *candidate == token)
                    }) {
                        return Err(Exception::throw_type(
                            &ctx,
                            "this Entity has been released and can no longer be mounted",
                        ));
                    }
                    let handle = runtime
                        .nested_view_handles
                        .borrow()
                        .get(&token)
                        .filter(|alias| alias.provenance.is_current())
                        .map(|alias| alias.handle)
                        .ok_or_else(|| {
                            Exception::throw_type(
                                &ctx,
                                "this Entity has been released and can no longer be mounted",
                            )
                        })?;
                    // Resolve and clone before borrowing the arena. The
                    // snapshot keeps this entity alive after handle release.
                    let view = { runtime.entities().view(handle) }.ok_or_else(|| {
                        Exception::throw_type(
                            &ctx,
                            "this Entity has been released and can no longer be mounted",
                        )
                    })?;
                    runtime
                        .arena
                        .borrow_mut()
                        .push_child_view(ChildViewSpec::new(handle, view))
                        .map_err(|error| Exception::throw_type(&ctx, &error.to_string()))
                }),
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

            // A `cx` for code the host is not calling with one in hand: the
            // `View` constructor, and `init` through it. Ambient, so it works
            // wherever a call is running and says so where none is.
            globals.set(
                "__async_cx",
                Func::from(async_context_object),
            )?;

            let attach_runtime = runtime.clone();
            globals.set(
                "__attach",
                Func::from(move |ctx: Ctx<'_>, id: u32, child: u32| -> JsResult<()> {
                    upgrade(&attach_runtime, &ctx)?.attach(&ctx, id, child)
                }),
            )?;

            let nullary_style_runtime = runtime.clone();
            globals.set(
                "__applyNullaryStyle",
                Func::from(move |ctx: Ctx<'_>, id: u32, index: u16| -> JsResult<()> {
                    let runtime = upgrade(&nullary_style_runtime, &ctx)?;
                    runtime.push_op_checked(&ctx, runtime.push_op(id, SpecOp::NullaryStyle(index)))
                }),
            )?;

            let param_style_runtime = runtime.clone();
            globals.set(
                "__applyParamStyle",
                Func::from(
                    move |ctx: Ctx<'_>, id: u32, index: usize, value: Opt<StyleArgument>| {
                        let runtime = upgrade(&param_style_runtime, &ctx)?;
                        runtime.apply_param_style(&ctx, id, index, value.0)
                    },
                ),
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

            // Test-only probes for `tests::benchmark`. Each one accepts a
            // prefix of `__apply`'s signature and does nothing with it, so the
            // difference between two of them is the cost of converting the one
            // argument that was added — and the difference between the last one
            // and `__apply` is everything `apply` itself does. There is no way
            // to measure that split from script alone: a crossing that does
            // nothing has to exist for the crossing to be priced.
            #[cfg(test)]
            {
                globals.set("__benchId", Func::from(|_id: u32| {}))?;
                globals.set("__benchName", Func::from(|_id: u32, _name: String| {}))?;
                globals.set(
                    "__benchArgs",
                    Func::from(|_id: u32, _name: String, _args: Arguments| {}),
                )?;
            }

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

    /// Adds an element to another element's children.
    fn attach(&self, ctx: &Ctx<'_>, id: SpecId, child: SpecId) -> JsResult<()> {
        // A `resizable_panel()` is not an element anywhere else: base's panel
        // reads its size out of the group's state and panics outright without
        // one. Refused here, where the script can be pointed at the line that
        // did it, rather than at paint time.
        let orphan = {
            let arena = self.arena.borrow();
            let component = |node| arena.node(node).and_then(crate::spec::SpecNode::component);
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

    /// Records a style method that takes an argument, addressed by index.
    ///
    /// The dispatch `apply` would do for the same call has already happened:
    /// the prelude closed over the position in the parametric table when it
    /// bound the method, so this resolves a name by indexing rather than by
    /// looking a string up.
    fn apply_param_style(
        &self,
        ctx: &Ctx<'_>,
        id: SpecId,
        index: usize,
        value: Option<StyleArgument>,
    ) -> JsResult<()> {
        let name = style::param_style_at(index)
            .ok_or_else(|| Exception::throw_type(ctx, "unknown element method"))?;
        let value = match value {
            Some(StyleArgument::Value(value)) => value,
            Some(StyleArgument::Handler) => {
                return Err(Exception::throw_type(
                    ctx,
                    &format!("`{name}` does not take a function"),
                ));
            }
            // Said by the same code that says it for every other bound method,
            // so a missing argument reads the same wherever it is missed.
            None => {
                let error = crate::value::arg(&[], 0, name)
                    .expect_err("argument 0 of an empty list is always missing");
                return Err(Exception::throw_type(ctx, error.message()));
            }
        };

        let args: SmallVec<[Bridged; 2]> = smallvec::smallvec![value];
        // Validate eagerly so a bad argument reports at the call site instead
        // of surfacing during materialize.
        style::apply_param(name, &args, Default::default())
            .map_err(|error| Exception::throw_type(ctx, error.message()))?;
        self.push_op_checked(ctx, self.push_op(id, SpecOp::ParamStyle(name, args)))
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
                self.attach(ctx, id, child)
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
            | "on_dismiss" | "on_step" | "on_item_click" | "on_mouse_move" | "on_hover" => {
                // A handler registered from inside a virtual list's item
                // renderer has nowhere to live. Callbacks belong to the
                // snapshot that registered them and are retired with it; the
                // snapshot outlives thousands of frames, while the rows are
                // rebuilt on every one — so twenty handlers a frame would
                // accumulate, unreachable and unreleased, for as long as the
                // description stood. Refused where it was written rather than
                // leaked quietly. `on_item_click` on the list is the one
                // handler that covers the rows, and it is registered from
                // `render()` like every other.
                if scope::current_phase() == Some(ScopePhase::Layout) {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!(
                            "`{method}` cannot be registered from a virtual list's item \
                             renderer: the rows are rebuilt every frame, so a handler \
                             registered there would pile up for as long as the view stood. \
                             Use `on_item_click((key, cx) => ...)` on the list itself, and \
                             read the row out of your own data with the stable key it gives you"
                        ),
                    ));
                }
                let saved = args.first_handler().ok_or_else(|| {
                    Exception::throw_type(ctx, &format!("{method}(handler) expects a function"))
                })?;
                let callback = self.callbacks.borrow_mut().push(CallbackEntry {
                    value: saved,
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                });
                let name = match method {
                    "on_click" => "on_click",
                    "on_mouse_move" => "on_mouse_move",
                    "on_hover" => "on_hover",
                    "on_item_click" => "on_item_click",
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
            | "tooltip"
            | "role"
            | "aria_selected"
            | "aria_active_descendant"
            | "track_focus"
            | "track_scroll"
            | "with_item_to_measure_index"
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
                    "tooltip" => "tooltip",
                    "role" => "role",
                    "aria_selected" => "aria_selected",
                    "aria_active_descendant" => "aria_active_descendant",
                    "track_focus" => "track_focus",
                    "track_scroll" => "track_scroll",
                    "with_item_to_measure_index" => "with_item_to_measure_index",
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
                // Dropping a non-string here would leave an element that
                // looks tooltipped and shows nothing on hover. It is also the
                // one place to say that the element form is not bound yet.
                if name == "tooltip"
                    && bridged
                        .first()
                        .and_then(|value| value.as_str().ok())
                        .is_none()
                {
                    return Err(Exception::throw_type(
                        ctx,
                        "tooltip(text) expects a string; a tooltip built from an element is not \
                         bound yet",
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
                if name == "size_range" {
                    let Some(min) = bridged.first().and_then(finite_number) else {
                        return Err(Exception::throw_range(
                            ctx,
                            "size_range minimum does not fit the native pixel range",
                        ));
                    };
                    let max = match bridged.get(1) {
                        Some(value) => Some(finite_number(value).ok_or_else(|| {
                            Exception::throw_range(
                                ctx,
                                "size_range maximum does not fit the native pixel range",
                            )
                        })?),
                        None => None,
                    };
                    if max.is_some_and(|max| max < min) {
                        return Err(Exception::throw_range(
                            ctx,
                            "size_range maximum must be greater than or equal to its minimum",
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

/// How far apart a virtualized list's items are: `number | number[]`.
///
/// Two forms rather than base's one vector, because the length of that vector
/// is also the item count — so mirroring it literally would put one number per
/// row across the language boundary on every script render. A hundred thousand
/// rows of a fixed height is one number here.
enum ItemExtents {
    Uniform(f64),
    PerItem(Vec<f64>),
}

impl<'js> FromJs<'js> for ItemExtents {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if let Some(uniform) = value.as_number() {
            return Ok(Self::Uniform(uniform));
        }
        let items = value.as_array().ok_or_else(|| {
            Exception::throw_type(ctx, "item_sizes must be a number or an array of numbers")
        })?;
        let length = native::bridge_array_len(ctx, &items).map_err(|_| {
            Exception::throw_range(
                ctx,
                &format!(
                    "item_sizes may contain at most {} entries",
                    native::MAX_BRIDGE_ARRAY_ITEMS
                ),
            )
        })?;
        let mut extents = Vec::new();
        extents.try_reserve_exact(length).map_err(|_| {
            Exception::throw_range(ctx, "the item_sizes array could not be allocated")
        })?;
        for extent in items.iter::<f64>() {
            extents.push(extent?);
        }
        Ok(Self::PerItem(extents))
    }
}

/// A script function taken as a constructor argument, saved on the way in.
///
/// Persisted inside `FromJs` for the reason [`Arguments`] gives: a closure
/// cannot unify the `Ctx<'js>` it takes with a borrowed value of the same
/// lifetime, so the crossing happens where both are still one lifetime.
struct ItemRenderer(Persistent<Function<'static>>);

impl<'js> FromJs<'js> for ItemRenderer {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let function = value.as_function().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "a virtual list needs a render function; it is called with the visible range,                  not once per item",
            )
        })?;
        Ok(Self(Persistent::save(ctx, function.clone())))
    }
}

/// A script function resolving one stable string key from a current index.
struct ItemKeyResolver(Persistent<Function<'static>>);

impl<'js> FromJs<'js> for ItemKeyResolver {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        let function = value.as_function().ok_or_else(|| {
            Exception::throw_type(
                ctx,
                "a virtual list needs get_key(index) to return each item's stable string key",
            )
        })?;
        Ok(Self(Persistent::save(ctx, function.clone())))
    }
}

/// The aggregate item count whose native extent vectors one render may own.
///
/// Base wants one `Size` per item and the vector is built here, so a count the
/// script fat-fingered — a byte offset, a timestamp — would be an allocation
/// measured in gigabytes before anything else had a chance to notice. This is
/// shared across every list in the description so several individually valid
/// lists cannot bypass it.
const MAX_VIRTUAL_ITEMS_PER_RENDER: usize = 1_000_000;

/// `v_virtual_list` and `h_virtual_list`.
///
/// The item renderer is registered as an ordinary callback, so it belongs to
/// the snapshot being built and is retired with it. That is also why it cannot
/// be registered from inside another item renderer: by then the generation that
/// would own it has been committed, and a callback pushed with no open
/// generation is one no lookup can ever match.
fn virtual_list_constructor(
    globals: &Object<'_>,
    name: &str,
    runtime: Weak<ShellRuntime>,
    axis: gpui::Axis,
) -> JsResult<()> {
    globals.set(
        name,
        Func::from(
            move |ctx: Ctx<'_>,
                  id: String,
                  count: usize,
                  extents: ItemExtents,
                  get_key: ItemKeyResolver,
                  render: ItemRenderer|
                  -> JsResult<SpecId> {
                if scope::current_phase() == Some(ScopePhase::Layout) {
                    return Err(Exception::throw_type(
                        &ctx,
                        "a virtual list cannot be built from inside another list's item                          renderer: its own renderer would belong to no render pass and would                          never be called. Describe the nested list from the view's render()                          instead",
                    ));
                }
                if !upgrade(&runtime, &ctx)?
                    .arena
                    .borrow_mut()
                    .claim_virtual_items(count, MAX_VIRTUAL_ITEMS_PER_RENDER)
                {
                    return Err(Exception::throw_type(
                        &ctx,
                        &format!(
                            "the virtual lists in one render may describe at most                              {MAX_VIRTUAL_ITEMS_PER_RENDER} items in total"
                        ),
                    ));
                }

                let extent = |value: f64| -> JsResult<gpui::Size<gpui::Pixels>> {
                    if !value.is_finite() || value < 0.0 {
                        return Err(Exception::throw_type(
                            &ctx,
                            "every item size must be a finite, non-negative number of pixels",
                        ));
                    }
                    // Only the extent along the list's own axis is read; the
                    // other is inferred by measuring one item. Writing zero
                    // there says so, rather than inventing a number base would
                    // ignore.
                    Ok(match axis {
                        gpui::Axis::Vertical => gpui::size(gpui::px(0.), gpui::px(value as f32)),
                        gpui::Axis::Horizontal => gpui::size(gpui::px(value as f32), gpui::px(0.)),
                    })
                };

                let reserve = |values: &mut Vec<gpui::Size<gpui::Pixels>>| {
                    values.try_reserve_exact(count).map_err(|_| {
                        Exception::throw_range(
                            &ctx,
                            "the virtual list's native size table could not be allocated",
                        )
                    })
                };
                let sizes = match extents {
                    ItemExtents::Uniform(value) => {
                        let value = extent(value)?;
                        let mut values = Vec::new();
                        reserve(&mut values)?;
                        values.resize(count, value);
                        values
                    }
                    ItemExtents::PerItem(values) => {
                        if values.len() != count {
                            return Err(Exception::throw_type(
                                &ctx,
                                &format!(
                                    "this list was given {} item sizes for {count} items; pass                                      one number for a uniform extent, or one per item",
                                    values.len()
                                ),
                            ));
                        }
                        let mut extents = Vec::new();
                        reserve(&mut extents)?;
                        for value in values {
                            extents.push(extent(value)?);
                        }
                        extents
                    }
                };

                let store = upgrade(&runtime, &ctx)?;
                let get_key = store.callbacks.borrow_mut().push(CallbackEntry {
                    value: get_key.0,
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                });
                let callback = store.callbacks.borrow_mut().push(CallbackEntry {
                    value: render.0,
                    view: scope::current_view().map(|view| view.downgrade()),
                    application: scope::current_application_generation(),
                });
                Ok(store.push_node(Component::VirtualList(Rc::new(
                    crate::spec::VirtualListSpec::new(
                        id,
                        axis,
                        Rc::new(sizes),
                        get_key,
                        callback,
                    ),
                ))))
            },
        ),
    )
}

/// The spec a `render` returned.
///
/// A retained child view counts, the way an `Entity<V>` is itself renderable in
/// GPUI: a view whose whole job is to hold one child should be able to say so
/// by returning it, rather than wrapping it in a container it does not want.
fn element_id(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<SpecId> {
    // A string is an element, so a view whose whole output is a word may say so
    // — `render` returns `impl IntoElement` in GPUI, and `&str` implements it.
    if value.as_string().is_some() {
        let make: rquickjs::Function = ctx.globals().get("__text")?;
        return make.call((value.get::<String>()?,));
    }
    let Some(object) = value.as_object() else {
        return Err(Exception::throw_type(
            ctx,
            "render(cx) must return an element or a string",
        ));
    };
    if let Ok(id) = object.get::<_, u32>("__id") {
        return Ok(id as SpecId);
    }
    if object.get::<_, bool>("__entity").unwrap_or(false) {
        let child_view: rquickjs::Function = ctx.globals().get("__child_view")?;
        let handle: u32 = object.get("__handle")?;
        return child_view.call((handle,));
    }
    value
        .as_object()
        .and_then(|object| object.get::<_, u32>("__id").ok())
        .ok_or_else(|| {
            Exception::throw_type(ctx, "render(cx) must return an element built with gpui")
        })
}

/// How a `cx` reaches the host call it speaks for.
///
/// GPUI draws this line with the borrow checker: `App` and `Context<T>` are
/// borrows that cannot outlive their call, and `AsyncApp` is the one flavor you
/// may hold across an `await`. A script has no borrow checker, so the same line
/// is drawn at run time — and it is the *only* difference between the two
/// kinds of `cx`. Every member gates on [`ContextBinding::check`] and then does
/// the same ambient work, so the two cannot drift apart.
#[derive(Clone, Copy)]
pub(crate) enum ContextBinding {
    /// One host call, named by its generation. Refuses once that call has
    /// returned, which is what catches a `cx` stashed in a closure and used
    /// from a later frame.
    Call(u64),
    /// Whichever host call is running now. Survives an `await`, because it
    /// names no frame that could go stale — the mirror of GPUI's `AsyncApp`.
    Ambient,
}

impl ContextBinding {
    /// Refuses a `cx` that cannot speak for a live call, before any member acts.
    fn check(self, ctx: &Ctx<'_>) -> JsResult<()> {
        self.with_app(ctx, |_| ())
    }

    /// The `App` of the call this `cx` speaks for.
    fn with_app<R>(self, ctx: &Ctx<'_>, body: impl FnOnce(&mut App) -> R) -> JsResult<R> {
        match self {
            Self::Call(generation) => scope::with_context(generation, |_, app| body(app))
                .map_err(|error| Exception::throw_type(ctx, &error.to_string())),
            Self::Ambient => scope::with_current_app(body).ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "this cx has no host call to speak for. An async cx works inside the task \
                     that owns it — from a handler the scheduler resumed — not from a bare \
                     promise callback or after the task was cancelled.",
                )
            }),
        }
    }
}

/// `globalThis.__async_cx()`. A free function rather than a closure because it
/// has to be generic over the JS lifetime.
fn async_context_object<'js>(ctx: Ctx<'js>) -> JsResult<Object<'js>> {
    context_object(&ctx, ContextBinding::Ambient)
}

/// The script-side `cx`.
///
/// It carries no state a script can reach — only the binding above, closed over
/// by the members — so `Object.keys(cx)` still shows nothing but methods and a
/// generation cannot be forged.
fn context_object<'js>(ctx: &Ctx<'js>, binding: ContextBinding) -> JsResult<Object<'js>> {
    let object = Object::new(ctx.clone())?;

    let module: Object = ctx.globals().get("__gpui")?;
    let members: Function = module.get("__context_members")?;
    let check = Func::from(move |ctx: Ctx<'_>| -> JsResult<()> { binding.check(&ctx) });
    let members: Object = members.call((check,))?;
    for name in members.keys::<String>() {
        let name = name?;
        let member: Value = members.get(&name as &str)?;
        object.set(name, member)?;
    }

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
            binding.with_app(&ctx, move |app| {
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

/// The single argument of a parametric style method.
///
/// Separate from [`Argument`] only because it arrives without an array around
/// it: a style takes one value, and building a JavaScript array to carry it was
/// measurable in the description pass. A function is carried as a marker rather
/// than saved, because no style takes one and the only thing left to do with it
/// is name the method that was handed one.
enum StyleArgument {
    Value(Bridged),
    Handler,
}

impl<'js> FromJs<'js> for StyleArgument {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if value.as_function().is_some() {
            return Ok(Self::Handler);
        }
        Ok(Self::Value(bridge(ctx, &value)?))
    }
}

/// Converts one non-function script value.
///
/// The one place the four bridged cases are named, so that [`Arguments`] and
/// [`StyleArgument`] cannot come to disagree about what a script value is.
fn bridge(ctx: &Ctx<'_>, value: &Value<'_>) -> JsResult<Bridged> {
    Ok(if value.is_null() || value.is_undefined() {
        Bridged::Nil
    } else if let Some(flag) = value.as_bool() {
        Bridged::Bool(flag)
    } else if let Some(number) = value.as_number() {
        Bridged::Number(number)
    } else if let Some(text) = value.as_string() {
        Bridged::Str(text.to_string()?)
    } else {
        return Err(Exception::throw_type(
            ctx,
            "unsupported argument type; expected null, boolean, number, string or function",
        ));
    })
}

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
            converted.push(match entry.as_function() {
                Some(handler) => Argument::Handler(Persistent::save(ctx, handler.clone())),
                None => Argument::Value(bridge(ctx, &entry)?),
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

fn refuse_nested_view_mutation(ctx: &Ctx<'_>, api: &str, action: &str) -> JsResult<()> {
    let Some(phase @ (ScopePhase::Render | ScopePhase::Layout)) = scope::current_phase() else {
        return Ok(());
    };
    Err(Exception::throw_type(
        ctx,
        &format!(
            "{api} cannot run during {}; {action} retained views from init(), an event handler \
             or a task",
            phase.as_str()
        ),
    ))
}

fn nested_view_needs_call(ctx: &Ctx<'_>, api: &str) -> JsError {
    Exception::throw_type(
        ctx,
        &format!("{api} needs a live host call; use it from init(), an event handler or a task"),
    )
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

#[cfg(test)]
mod nested_view_lifecycle_tests {
    use super::*;
    use gpui::{ClickEvent, TestAppContext, VisualTestContext};
    use rquickjs::{Object, Persistent};

    struct ChildMount(Entity<ScriptView>);

    impl gpui::Render for ChildMount {
        fn render(
            &mut self,
            _: &mut Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            self.0.clone()
        }
    }

    fn child_type(runtime: &Rc<ShellRuntime>, source: &str) -> ViewType {
        let mut view_type = runtime
            .load_source("nested-child.js", source)
            .expect("load child view");
        view_type.application = Some(ApplicationGeneration::new(7));
        view_type
    }

    #[gpui::test]
    fn foreign_release_cannot_probe_or_remove_a_dead_nested_alias(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let owner_application = ApplicationGeneration::new(71);
        let foreign_application = ApplicationGeneration::new(72);
        let owner_policy = Rc::new(Policy::default());
        let foreign_policy = Rc::new(Policy::default());
        let mut view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
export default class Child extends View { render(cx) { return "child"; } }
"#,
        );
        view_type.application = Some(owner_application.clone());
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(&view_type, owner_policy.clone(), None, window, cx)
            })
            .expect("child");
        let token = 91;
        runtime.nested_view_handles.borrow_mut().insert(
            token,
            NestedViewAlias {
                handle,
                provenance: NestedViewProvenance {
                    application: Some(owner_application.clone()),
                    policy: owner_policy.clone(),
                },
            },
        );
        let release = runtime
            .entities()
            .release_view(handle)
            .expect("typed child release");
        context.update(|_, cx| release.retire(cx));
        assert!(runtime.entities().view(handle).is_none());

        let foreign = context.update(|window, cx| {
            let (_scope, _) = scope::enter_with_application(
                &runtime,
                window,
                cx,
                ScopePhase::Event,
                None,
                foreign_policy,
                Some(foreign_application),
            );
            runtime.with_js(|ctx| runtime.queue_nested_view_release(ctx, token))
        });
        assert!(foreign.is_err(), "foreign authority must be rejected");
        assert!(
            runtime.nested_view_handles.borrow().contains_key(&token),
            "foreign release observed liveness and removed the dead alias"
        );

        let owner = context.update(|window, cx| {
            let (_scope, _) = scope::enter_with_application(
                &runtime,
                window,
                cx,
                ScopePhase::Event,
                None,
                owner_policy,
                Some(owner_application),
            );
            runtime.with_js(|ctx| runtime.queue_nested_view_release(ctx, token))
        });
        assert_eq!(owner.expect("owner call"), false);
        assert!(!runtime.nested_view_handles.borrow().contains_key(&token));
    }

    #[gpui::test]
    fn releasing_a_rendered_child_retires_callbacks_while_a_frame_retains_it(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { View, div } from "gpui";
globalThis.child_hits = 0;

export default class Child extends View {
  render(cx) {
    return div()
      .on_click(() => { globalThis.child_hits += 1; })
      .child("child");
  }
}
"#,
        );
        let runtime_for_window = runtime.clone();
        let handle_slot = Rc::new(Cell::new(None));
        let handle_for_window = handle_slot.clone();
        let window = cx.add_window(move |window, cx| {
            let handle = runtime_for_window
                .instantiate_nested_view(&view_type, crate::policy::default(), None, window, cx)
                .expect("child");
            handle_for_window.set(Some(handle));
            ChildMount(
                runtime_for_window
                    .entities()
                    .view(handle)
                    .expect("retained child"),
            )
        });
        let mut context = VisualTestContext::from_window(*window, cx);
        context.update(|window, cx| window.draw(cx).clear(cx));

        let handle = handle_slot.get().expect("child handle");
        let retained_frame = runtime.entities().view(handle).expect("frame entity clone");
        let callback = runtime
            .live_callback_ids()
            .into_iter()
            .next()
            .expect("rendered click callback");
        assert_eq!(runtime.live_callbacks(), 1);

        assert!(context.update(|_, cx| runtime.release_view_handle(handle, cx)));
        assert_eq!(
            runtime.live_callbacks(),
            0,
            "release must retire current and previous callback generations immediately"
        );
        context.update(|window, cx| {
            runtime.dispatch_click(callback, &ClickEvent::default(), window, cx)
        });
        let hits = runtime
            .with_js(|ctx| ctx.globals().get::<_, usize>("child_hits"))
            .expect("child hit count");
        assert_eq!(hits, 0, "a released callback must be inert");
        assert!(runtime.entities().view(handle).is_none());
        assert!(
            !context.update(|_, cx| runtime.release_view_handle(handle, cx)),
            "typed release must reject a stale view handle"
        );

        drop(retained_frame);
        context.update(|window, _| window.remove_window());
        context.run_until_parked();
        drop(context);
        let weak_runtime = Rc::downgrade(&runtime);
        drop(runtime);
        assert!(
            weak_runtime.upgrade().is_none(),
            "retired callbacks must not keep the child and runtime in a cycle"
        );
    }

    #[gpui::test]
    fn nested_init_receives_props_after_the_final_entity_exists(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";

export default class Child extends View {
  init(props, cx) {
    this.label = props.label;
    this.tick = cx.timer.every(60_000, () => {});
    Promise.resolve().then(() => {
      this.continuation_input = InputState.new({ value: "continued" });
      this.continued = true;
    });
  }
  render(cx) { return this.label; }
}
"#,
        );
        let props = runtime
            .with_js(|ctx| {
                let props = Object::new(ctx.clone())?;
                props.set("label", "from props")?;
                Ok(Persistent::save(ctx, props.into_value()))
            })
            .expect("props");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let tasks_before = task_count();
        let records_before = runtime.entities().len();

        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    Some(props),
                    window,
                    cx,
                )
            })
            .expect("instantiate nested child");

        let view = runtime
            .entities()
            .view(handle)
            .expect("the returned handle retains the child entity");
        assert!(
            runtime.entities().focus(handle).is_none(),
            "a view handle must never resolve as another retained type"
        );
        let object = context.update(|_, cx| view.read(cx).object().clone());
        let label = runtime
            .with_js(|ctx| object.clone().restore(ctx)?.get::<_, String>("label"))
            .expect("read initialized label");
        assert_eq!(label, "from props");
        let continued = runtime
            .with_js(|ctx| object.clone().restore(ctx)?.get::<_, bool>("continued"))
            .expect("read init continuation marker");
        assert!(
            continued,
            "successful init promise jobs must drain before the child scope exits"
        );
        assert_eq!(
            runtime.entities().len(),
            records_before + 2,
            "the continuation's retained state must be owned beside its child view"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "init work must be registered under the final child owner"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(handle, cx)));
        assert_eq!(
            task_count(),
            tasks_before,
            "releasing the handle must cancel its exact-owner task even while a frame retains the entity"
        );
        assert_eq!(runtime.entities().len(), records_before);
        drop(view);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn successful_child_init_does_not_claim_a_preexisting_parent_job(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let parent_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";
globalThis.parent_continuations = 0;
// Takes a context, because module scope has none: the caller is a live host
// call and hands one in, and the async flavour is still usable when the drain
// runs this `.then` later.
globalThis.queue_parent_job = (cx) => Promise.resolve().then(() => {
  globalThis.parent_continuations += 1;
  globalThis.parent_input = InputState.new({ value: "parent" });
  globalThis.parent_tick = cx.timer.every(60_000, () => {});
});
export default class Parent extends View {
  render() { return "parent"; }
}
"#,
        );
        let child_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";
globalThis.child_continuations = 0;
export default class Child extends View {
  init(_props, cx) {
    Promise.resolve().then(() => {
      globalThis.child_continuations += 1;
      this.input = InputState.new({ value: "child" });
      this.tick = cx.timer.every(60_000, () => {});
    });
  }
  render(cx) { return "child"; }
}
"#,
        );
        let application = parent_type.application.clone();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let records_before = runtime.entities().len();
        let tasks_before = task_count();
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &parent_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let child = context
            .update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Event,
                    Some(parent_entity.clone()),
                    crate::policy::default(),
                    application.clone(),
                );
                runtime
                    .with_js(|ctx| {
                        ctx.globals()
                            .get::<_, Function>("queue_parent_job")?
                            .call::<_, ()>((context_object(ctx, ContextBinding::Ambient)?,))
                    })
                    .expect("queue parent continuation");
                runtime.instantiate_nested_view(
                    &child_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("child");

        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("parent_continuations"))
                .expect("parent count"),
            1
        );
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("child_continuations"))
                .expect("child count"),
            1
        );
        assert_eq!(runtime.entities().len(), records_before + 4);
        assert_eq!(task_count(), tasks_before + 2);

        assert!(context.update(|_, cx| runtime.release_view_handle(child, cx)));
        assert_eq!(
            runtime.entities().len(),
            records_before + 2,
            "child release must preserve the parent view and its continuation-owned input"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "child release must preserve the parent continuation-owned timer"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        assert_eq!(runtime.entities().len(), records_before);
        assert_eq!(task_count(), tasks_before);
        drop(parent_entity);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn throwing_child_init_rolls_back_its_job_but_not_a_preexisting_parent_job(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let parent_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";
globalThis.parent_continuations = 0;
// Takes a context, because module scope has none: the caller is a live host
// call and hands one in, and the async flavour is still usable when the drain
// runs this `.then` later.
globalThis.queue_parent_job = (cx) => Promise.resolve().then(() => {
  globalThis.parent_continuations += 1;
  globalThis.parent_input = InputState.new({ value: "parent" });
  globalThis.parent_tick = cx.timer.every(60_000, () => {});
});
export default class Parent extends View {
  render() { return "parent"; }
}
"#,
        );
        let child_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";
globalThis.child_continuations = 0;
export default class BrokenChild extends View {
  init(_props, cx) {
    Promise.resolve().then(() => {
      globalThis.child_continuations += 1;
      this.input = InputState.new({ value: "child" });
      this.tick = cx.timer.every(60_000, () => {});
    });
    throw new Error("mixed init failed");
  }
  render(cx) { return "unreachable"; }
}
"#,
        );
        let application = parent_type.application.clone();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let records_before = runtime.entities().len();
        let tasks_before = task_count();
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &parent_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let error = context
            .update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Event,
                    Some(parent_entity.clone()),
                    crate::policy::default(),
                    application.clone(),
                );
                runtime
                    .with_js(|ctx| {
                        ctx.globals()
                            .get::<_, Function>("queue_parent_job")?
                            .call::<_, ()>((context_object(ctx, ContextBinding::Ambient)?,))
                    })
                    .expect("queue parent continuation");
                runtime.instantiate_nested_view(
                    &child_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect_err("child init must fail");

        assert!(error.to_string().contains("mixed init failed"), "{error}");
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("parent_continuations"))
                .expect("parent count"),
            1
        );
        assert_eq!(
            runtime
                .with_js(|ctx| ctx.globals().get::<_, usize>("child_continuations"))
                .expect("child count"),
            1
        );
        assert_eq!(
            runtime.entities().len(),
            records_before + 2,
            "failed child rollback must preserve the parent view and continuation-owned input"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "failed child rollback must preserve the parent continuation-owned timer"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        assert_eq!(runtime.entities().len(), records_before);
        assert_eq!(task_count(), tasks_before);
        drop(parent_entity);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn successor_first_init_chain_fails_the_runtime_and_rolls_back_the_child(
        cx: &mut TestAppContext,
    ) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let parent_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
export default class Parent extends View {
  render() { return "parent"; }
}
"#,
        );
        let child_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";
globalThis.successor_runs = 0;
export default class BrokenChild extends View {
  init(_props, cx) {
    this.input = InputState.new({ value: "candidate" });
    this.tick = cx.timer.every(60_000, () => {});
    const again = () => {
      Promise.resolve().then(again);
      Promise.resolve().then(again);
      globalThis.successor_runs += 1;
    };
    Promise.resolve().then(again);
  }
  render(cx) { return "unreachable"; }
}
"#,
        );
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &parent_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let records_before = runtime.entities().len();
        let tasks_before = task_count();

        let error = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &child_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect_err("a non-quiescing init wave must fail within the host job bound");

        assert!(error.to_string().contains("job queue"), "{error}");
        assert_eq!(
            runtime.entities().len(),
            records_before,
            "terminal job failure must roll back the candidate child locally"
        );
        assert_eq!(task_count(), tasks_before);
        context.update(|window, cx| {
            scheduler::drain_after_render(
                &runtime,
                parent_entity.clone(),
                crate::policy::default(),
                window,
                cx,
            )
        });
        assert_eq!(
            task_count(),
            tasks_before,
            "terminal pending jobs must not register a later deferred drain"
        );
        let disabled = context.update(|window, cx| {
            runtime.instantiate_nested_view(&child_type, crate::policy::default(), None, window, cx)
        });
        assert!(
            disabled
                .expect_err("the failed runtime must refuse later script execution")
                .to_string()
                .contains("job queue")
        );
        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        drop(parent_entity);
    }

    #[gpui::test]
    fn nested_view_retains_the_real_loaded_application_lease(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-nested-view-lease-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("application directory");
        std::fs::write(
            root.join("main.js"),
            r#"
import { div, View } from "gpui";
export default class Child extends View {
  render(cx) { return "loaded child"; }
}
"#,
        )
        .expect("application source");
        let view_type = runtime.load_app(&root, "main.js").expect("loaded app");
        let application = view_type
            .application
            .clone()
            .expect("real application lease");
        assert_eq!(runtime.app_modules.registration_count(), 1);
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let handle = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("nested loaded child");
        let view = runtime.entities().view(handle).expect("retained child");
        let child_application = context.update(|_, cx| {
            view.read(cx)
                .application_generation()
                .expect("child application")
        });
        assert!(Rc::ptr_eq(&child_application, &application));

        drop(view_type);
        assert_eq!(
            runtime.app_modules.registration_count(),
            1,
            "the retained child object must keep its evaluated module lease"
        );
        context.update(|_, cx| runtime.release_application_generation(&application, cx));
        cancel_application_tasks(&application);
        assert!(
            runtime.entities().view(handle).is_none(),
            "application unload must remove the child's retained handle"
        );
        drop(view);
        drop(child_application);
        drop(application);
        context.update(|_, _| {});
        assert_eq!(runtime.app_modules.registration_count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[gpui::test]
    fn releasing_one_child_preserves_its_sibling_and_application_state(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";

export default class Child extends View {
  init(_props, cx) {
    this.input = InputState.new();
    this.tick = cx.timer.every(60_000, () => {});
  }
  render(cx) { return "child"; }
}
"#,
        );
        let application = view_type.application.clone().expect("application");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let tasks_before = task_count();
        let application_focus =
            context.update(|_, cx| runtime.entities().create_focus(Some(application), cx));
        assert!(
            !context.update(|_, cx| runtime.release_view_handle(application_focus, cx)),
            "typed view release must reject a live handle of another retained type"
        );
        let first = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("first child");
        let second = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("second child");

        assert_eq!(task_count(), tasks_before + 2);
        assert!(context.update(|_, cx| runtime.release_view_handle(first, cx)));
        context.update(|_, _| {});

        assert!(runtime.entities().view(first).is_none());
        assert!(
            runtime.entities().view(second).is_some(),
            "releasing one child must preserve its sibling"
        );
        assert!(
            runtime.entities().focus(application_focus).is_some(),
            "nested cleanup must not release application-owned retained state"
        );
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "nested cleanup must cancel only the released child's task"
        );

        assert!(context.update(|_, cx| runtime.release_view_handle(second, cx)));
        assert!(runtime.entities().release(application_focus));
        context.update(|_, _| {});
        assert_eq!(task_count(), tasks_before);
    }

    #[gpui::test]
    fn releasing_a_child_recursively_cancels_retained_descendant_tasks(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";

export default class Child extends View {
  init(_props, cx) { this.tick = cx.timer.every(60_000, () => {}); }
  render(cx) { return "child"; }
}
"#,
        );
        let application = view_type.application.clone();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let tasks_before = task_count();
        let parent = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("parent child");
        let parent_entity = runtime.entities().view(parent).expect("parent entity");
        let descendant = context
            .update(|window, cx| {
                let (_scope, _) = scope::enter_with_application(
                    &runtime,
                    window,
                    cx,
                    ScopePhase::Event,
                    Some(parent_entity.clone()),
                    crate::policy::default(),
                    application.clone(),
                );
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("descendant child");
        let retained_descendant = runtime
            .entities()
            .view(descendant)
            .expect("descendant entity clone");

        assert_eq!(task_count(), tasks_before + 2);
        assert!(context.update(|_, cx| runtime.release_view_handle(parent, cx)));
        assert!(runtime.entities().view(parent).is_none());
        assert!(runtime.entities().view(descendant).is_none());
        assert_eq!(
            task_count(),
            tasks_before,
            "subtree cleanup must cancel descendant tasks even while GPUI retains their entities"
        );

        drop(retained_descendant);
        drop(parent_entity);
        context.update(|_, _| {});
    }

    #[gpui::test]
    fn exact_view_cancellation_is_qualified_by_runtime_across_apps(cx: &mut TestAppContext) {
        let runtime_a = ShellRuntime::new_isolated().expect("first runtime");
        let runtime_b = ShellRuntime::new_isolated().expect("second runtime");
        let view_type_a = child_type(
            &runtime_a,
            r#"
import { div, View } from "gpui";
export default class Child extends View {
  init(_props, cx) { this.tick = cx.timer.every(60_000, () => {}); }
  render() { return "a"; }
}
"#,
        );
        let view_type_b = child_type(
            &runtime_b,
            r#"
import { div, View } from "gpui";
export default class Child extends View {
  init(_props, cx) { this.tick = cx.timer.every(60_000, () => {}); }
  render(cx) { return "b"; }
}
"#,
        );
        let mut other = cx.new_app();
        let window_a = cx.add_window(|_, _| gpui::Empty);
        let window_b = other.add_window(|_, _| gpui::Empty);
        let mut context_a = VisualTestContext::from_window(*window_a, cx);
        let mut context_b = VisualTestContext::from_window(*window_b, &mut other);
        let tasks_before = task_count();
        let handle_a = context_a
            .update(|window, cx| {
                runtime_a.instantiate_nested_view(
                    &view_type_a,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("first child");
        let handle_b = context_b
            .update(|window, cx| {
                runtime_b.instantiate_nested_view(
                    &view_type_b,
                    crate::policy::default(),
                    None,
                    window,
                    cx,
                )
            })
            .expect("second child");
        let entity_a = runtime_a.entities().view(handle_a).expect("first entity");
        let entity_b = runtime_b.entities().view(handle_b).expect("second entity");

        assert_eq!(
            entity_a.entity_id(),
            entity_b.entity_id(),
            "fresh Apps must reproduce the local EntityId collision exercised by this test"
        );
        assert!(
            !context_b.update(|_, cx| runtime_b.release_view_handle(handle_a, cx)),
            "typed release must reject a handle from another runtime's store"
        );
        assert!(runtime_b.entities().view(handle_b).is_some());
        assert_eq!(task_count(), tasks_before + 2);
        assert!(context_a.update(|_, cx| runtime_a.release_view_handle(handle_a, cx)));
        assert_eq!(
            task_count(),
            tasks_before + 1,
            "releasing one App's colliding EntityId must preserve the other runtime's task"
        );
        assert!(runtime_b.entities().view(handle_b).is_some());

        assert!(context_b.update(|_, cx| runtime_b.release_view_handle(handle_b, cx)));
        assert_eq!(task_count(), tasks_before);
        drop(entity_a);
        drop(entity_b);
    }

    #[gpui::test]
    fn failed_child_init_rolls_back_only_the_candidate_child(cx: &mut TestAppContext) {
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let view_type = child_type(
            &runtime,
            r#"
import { div, View } from "gpui";
import { InputState } from "gpui-base";
globalThis.failed_child_continuations = 0;

export default class BrokenChild extends View {
  init(props, cx) {
    this.input = InputState.new({ value: props.value });
    this.tick = cx.timer.every(60_000, () => {});
    Promise.resolve().then(() => {
      globalThis.failed_child_continuations += 1;
      globalThis.continuation_input = InputState.new({ value: "continued" });
    });
    throw new Error("child init failed");
  }
  render(cx) { return "unreachable"; }
}
"#,
        );
        let application = view_type.application.clone().expect("application");
        let props = runtime
            .with_js(|ctx| {
                let props = Object::new(ctx.clone())?;
                props.set("value", "candidate")?;
                Ok(Persistent::save(ctx, props.into_value()))
            })
            .expect("props");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window, cx);
        let application_focus =
            context.update(|_, cx| runtime.entities().create_focus(Some(application), cx));
        let records_before = runtime.entities().len();
        let tasks_before = task_count();

        let error = context
            .update(|window, cx| {
                runtime.instantiate_nested_view(
                    &view_type,
                    crate::policy::default(),
                    Some(props),
                    window,
                    cx,
                )
            })
            .expect_err("child init must fail");

        assert!(error.to_string().contains("child init failed"), "{error}");
        assert_eq!(
            runtime.entities().len(),
            records_before,
            "the child handle and retained state created by init must roll back"
        );
        assert!(
            runtime.entities().focus(application_focus).is_some(),
            "rollback must preserve application-owned state"
        );
        assert_eq!(
            task_count(),
            tasks_before,
            "rollback must cancel the candidate child's exact-owner task"
        );
        let continuations = runtime
            .with_js(|ctx| ctx.globals().get::<_, usize>("failed_child_continuations"))
            .expect("continuation count");
        assert_eq!(
            continuations, 1,
            "init promise jobs must drain while the candidate child still owns the scope"
        );

        context.update(|window, cx| scheduler::drain_runtime_jobs(&runtime, window, cx));
        assert_eq!(runtime.entities().len(), records_before);

        assert!(runtime.entities().release(application_focus));
    }
}
