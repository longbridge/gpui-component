//! The Lua engine: module registration, method dispatch, callbacks, and the
//! render entry point. Kept behind the `lua`/`luajit` feature as the fallback
//! engine described in `engine/mod.rs`.
//!
//! One `ShellRuntime` owns one Lua state. It is main-thread only, like GPUI's
//! `App`, and is reached from anywhere through an `App` global.

use std::{
    cell::{RefCell, RefMut},
    path::Path,
    rc::{Rc, Weak},
};

use anyhow::{Context as _, Result, anyhow};
use gpui::{AnyElement, App, ClickEvent, Entity, Global, Window};
use mlua::{
    AnyUserData, Error as LuaError, Function, Lua, MetaMethod, MultiValue, Result as LuaResult,
    Table, UserData, UserDataMethods, Value as LuaValue,
};
use smallvec::SmallVec;

use crate::{
    materialize::materialize,
    runtime::{CallbackArena, CallbackEntry, error_overlay},
    scope::{self, ScopePhase},
    spec::{CallbackId, Component, SpecArena, SpecId, SpecOp},
    style,
    value::Bridged,
    view::ScriptView,
};

/// A script value that defines a view type.
pub type ViewType = Table;
/// One instance of a view type.
pub type ViewObject = Table;

pub struct ShellRuntime {
    lua: Lua,
    arena: RefCell<SpecArena>,
    callbacks: RefCell<CallbackArena<Function>>,
}

struct RuntimeGlobal(Rc<ShellRuntime>);

impl Global for RuntimeGlobal {}

impl ShellRuntime {
    /// Creates a VM with the `gpui` module installed.
    pub fn new() -> Result<Rc<Self>> {
        let runtime = Rc::new(Self {
            lua: Lua::new(),
            arena: RefCell::new(SpecArena::new()),
            callbacks: RefCell::new(CallbackArena::default()),
        });
        install_method_cache(&runtime.lua).map_err(host_error)?;
        runtime.install_module().map_err(host_error)?;
        Ok(runtime)
    }

    /// Stores the runtime so elements and callbacks can find it later.
    pub fn set_global(self: &Rc<Self>, cx: &mut App) {
        cx.set_global(RuntimeGlobal(self.clone()));
    }

    pub fn global(cx: &App) -> Option<Rc<Self>> {
        cx.try_global::<RuntimeGlobal>().map(|g| g.0.clone())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    pub fn arena_mut(&self) -> RefMut<'_, SpecArena> {
        self.arena.borrow_mut()
    }

    /// Loads `main.lua` from an application directory and returns the view type
    /// it evaluates to.
    pub fn load_app(self: &Rc<Self>, dir: &Path, entry: &str) -> Result<Table> {
        let root = crate::runtime::resolve_app_root(dir, entry)?;
        self.set_module_search_path(&root).map_err(host_error)?;

        let entry = root.join(entry);
        let source = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        let value: LuaValue = self
            .lua
            .load(&source)
            .set_name(entry.to_string_lossy().as_ref())
            .eval()
            .map_err(host_error)?;

        match value {
            LuaValue::Table(table) => Ok(table),
            other => Err(anyhow!(
                "main.lua must return a view type created with gpui.view(name), got {}",
                other.type_name()
            )),
        }
    }

    /// Points `require` at the application directory, and only there.
    fn set_module_search_path(&self, dir: &Path) -> LuaResult<()> {
        let package: Table = self.lua.globals().get("package")?;
        package.set(
            "path",
            format!("{dir}/?.lua;{dir}/?/init.lua", dir = dir.display()),
        )
    }

    /// Loads Lua source directly. Used by tests and by hosts that embed a
    /// script rather than an application directory.
    pub fn load_source(self: &Rc<Self>, name: &str, source: &str) -> Result<Table> {
        let value: LuaValue = self
            .lua
            .load(source)
            .set_name(name)
            .eval()
            .map_err(host_error)?;

        match value {
            LuaValue::Table(table) => Ok(table),
            other => Err(anyhow!(
                "expected a view type created with gpui.view(name), got {}",
                other.type_name()
            )),
        }
    }

    /// Renders a view and returns the element description as text, without
    /// materializing it.
    ///
    /// The description is plain data, so interface structure can be asserted in
    /// tests that never paint a frame. This is the seam that keeps Lua UI
    /// regression-testable.
    pub fn render_to_spec(
        self: &Rc<Self>,
        object: &Table,
        view: Option<Entity<ScriptView>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<String> {
        self.arena.borrow_mut().reset();
        self.callbacks.borrow_mut().swap();

        let root = {
            let (_guard, generation) = scope::enter(window, cx, ScopePhase::Render, view);
            render_once(object, generation).map_err(host_error)?
        };

        Ok(self.arena.borrow().debug_tree(root))
    }

    /// Instantiates a view type, running its `init`.
    ///
    /// `init` is where a view creates the state it keeps across frames, and
    /// creating an entity needs a `Window` and an `App`. So construction opens
    /// a scope of its own rather than running in the gap between host calls.
    pub fn instantiate(
        self: &Rc<Self>,
        view_type: &Table,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Table> {
        let new: Function = view_type
            .get("new")
            .map_err(|_| anyhow!("value returned by main.lua is not a gpui.view type"))?;

        let (_guard, _generation) = scope::enter(window, cx, ScopePhase::Event, None);
        new.call::<Table>(LuaValue::Nil).map_err(host_error)
    }

    /// Renders one Lua view: reset the arena, call `render`, materialize.
    pub fn render_view(
        self: &Rc<Self>,
        object: Table,
        entity: Entity<ScriptView>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        self.arena.borrow_mut().reset();
        self.callbacks.borrow_mut().swap();

        let spec = {
            let (_guard, generation) =
                scope::enter(window, cx, ScopePhase::Render, Some(entity.clone()));
            render_once(&object, generation)
        };

        match spec {
            Ok(id) => materialize(self, id, window, cx),
            Err(error) => error_overlay(&error.to_string(), window, cx),
        }
    }

    /// Invokes a click handler inside a fresh event scope.
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

        let arguments = match click_event_table(&self.lua, event) {
            Ok(table) => table,
            Err(error) => {
                tracing::error!("building click event table failed: {error}");
                return;
            }
        };

        if let Err(error) = entry
            .value
            .call::<()>((arguments, LuaContext { generation }))
        {
            tracing::error!("error in click handler: {error}");
        }
    }

    /// Invokes a controlled-value handler inside a fresh event scope.
    ///
    /// Base controls report intent rather than mutating their own state, so the
    /// new value is passed to Lua and it is Lua's job to store it and notify.
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

        if let Err(error) = entry.value.call::<()>((checked, LuaContext { generation })) {
            tracing::error!("error in change handler: {error}");
        }
    }

    fn push_callback(&self, function: Function) -> CallbackId {
        self.callbacks.borrow_mut().push(CallbackEntry {
            value: function,
            view: scope::current_view(),
        })
    }

    fn push_node(self: &Rc<Self>, component: Component) -> LuaResult<Element> {
        let id = self.arena.borrow_mut().push(component);
        Ok(Element {
            id,
            runtime: Rc::downgrade(self),
        })
    }

    /// Applies one recorded builder call.
    fn apply(self: &Rc<Self>, id: SpecId, method: &str, args: MultiValue) -> LuaResult<()> {
        match method {
            "child" => {
                let child = element_argument(&args, 0, method)?;
                self.arena
                    .borrow_mut()
                    .attach(id, child)
                    .map_err(|error| LuaError::runtime(error.to_string()))
            }
            "children" => {
                let LuaValue::Table(list) = args.front().cloned().unwrap_or(LuaValue::Nil) else {
                    return Err(LuaError::runtime(
                        "`children` expects a table of elements".to_string(),
                    ));
                };
                for entry in list.sequence_values::<LuaValue>() {
                    let child = element_id(&entry?)?;
                    self.arena
                        .borrow_mut()
                        .attach(id, child)
                        .map_err(|error| LuaError::runtime(error.to_string()))?;
                }
                Ok(())
            }
            "on_click" => {
                let LuaValue::Function(handler) = args.front().cloned().unwrap_or(LuaValue::Nil)
                else {
                    return Err(LuaError::runtime(
                        "`on_click` expects a function".to_string(),
                    ));
                };
                let callback = self.push_callback(handler);
                self.push_op(id, SpecOp::Callback("on_click", callback))
            }
            "on_change" => {
                let LuaValue::Function(handler) = args.front().cloned().unwrap_or(LuaValue::Nil)
                else {
                    return Err(LuaError::runtime(
                        "`on_change` expects a function".to_string(),
                    ));
                };
                let callback = self.push_callback(handler);
                self.push_op(id, SpecOp::Callback("on_change", callback))
            }
            "disabled" | "selected" | "checked" => {
                let bridged = bridge_args(&args)?;
                let name = match method {
                    "disabled" => "disabled",
                    "selected" => "selected",
                    _ => "checked",
                };
                self.push_op(id, SpecOp::Method(name, bridged))
            }
            _ => {
                if let Some(index) = style::nullary_index(method) {
                    return self.push_op(id, SpecOp::NullaryStyle(index));
                }
                if let Some(name) = style::param_style_name(method) {
                    let bridged = bridge_args(&args)?;
                    // Validate eagerly so a bad argument points at the call
                    // site rather than surfacing during materialize.
                    style::apply_param(name, &bridged, Default::default())
                        .map_err(|error| LuaError::runtime(error.message()))?;
                    return self.push_op(id, SpecOp::ParamStyle(name, bridged));
                }
                Err(LuaError::runtime(unknown_method_message(method)))
            }
        }
    }

    fn push_op(&self, id: SpecId, op: SpecOp) -> LuaResult<()> {
        self.arena
            .borrow_mut()
            .push_op(id, op)
            .map_err(|error| LuaError::runtime(error.to_string()))
    }

    fn install_module(self: &Rc<Self>) -> LuaResult<()> {
        let lua = &self.lua;
        let module = lua.create_table()?;

        for (name, component) in [
            ("div", Component::Div),
            ("h_flex", Component::HFlex),
            ("v_flex", Component::VFlex),
        ] {
            let runtime = Rc::downgrade(self);
            let component = component.clone();
            let constructor = lua.create_function(move |_, ()| {
                let runtime = upgrade(&runtime)?;
                runtime.push_node(component.clone())
            })?;
            module.set(name, constructor)?;
        }

        let runtime = Rc::downgrade(self);
        module.set(
            "text",
            lua.create_function(move |_, value: String| {
                upgrade(&runtime)?.push_node(Component::Text(value))
            })?,
        )?;

        for (name, build) in [
            ("Button", Component::Button as fn(String) -> Component),
            ("Checkbox", Component::Checkbox as fn(String) -> Component),
            ("Switch", Component::Switch as fn(String) -> Component),
        ] {
            let type_table = lua.create_table()?;
            let runtime = Rc::downgrade(self);
            type_table.set(
                "new",
                lua.create_function(move |_, id: String| upgrade(&runtime)?.push_node(build(id)))?,
            )?;
            type_table.set("__type_name", name)?;
            module.set(name, type_table)?;
        }

        module.set("view", lua.create_function(define_view)?)?;
        module.set(
            "style_names",
            lua.create_function(|_, ()| Ok(style::known_names().join(" ")))?,
        )?;

        let preload: Table = lua.globals().get::<Table>("package")?.get("preload")?;
        let module_value = module.clone();
        preload.set(
            "gpui",
            lua.create_function(move |_, ()| Ok(module_value.clone()))?,
        )?;
        lua.globals().set("gpui", module)?;

        Ok(())
    }
}

/// `gpui.view(name)` — defines a view type whose instances carry Lua state.
fn define_view(lua: &Lua, name: String) -> LuaResult<Table> {
    let view_type = lua.create_table()?;
    view_type.set("__view_name", name)?;
    view_type.set("__index", &view_type)?;

    let type_for_new = view_type.clone();
    let new = lua.create_function(move |lua, properties: LuaValue| {
        let instance = lua.create_table()?;
        instance.set_metatable(Some(type_for_new.clone()))?;
        if let Ok(init) = type_for_new.get::<Function>("init") {
            init.call::<()>((instance.clone(), properties))?;
        }
        Ok(instance)
    })?;
    view_type.set("new", new)?;

    Ok(view_type)
}

fn render_once(object: &Table, generation: u64) -> LuaResult<SpecId> {
    let render: Function = object.get("render").map_err(|_| {
        LuaError::runtime("view type has no `render` method; define function T:render(cx)")
    })?;
    let value: LuaValue = render.call((object.clone(), LuaContext { generation }))?;
    element_id(&value)
}

/// mlua's error type is neither `Send` nor `Sync`, so it cannot cross into
/// `anyhow` with `?`. Flattening it to a message at the host boundary keeps the
/// script's traceback intact while giving callers an ordinary error.
fn host_error(error: LuaError) -> anyhow::Error {
    anyhow!("{error}")
}

/// The Lua-side element handle. It carries a spec id, nothing more: the
/// description itself lives in the arena.
pub struct Element {
    id: SpecId,
    runtime: Weak<ShellRuntime>,
}

impl UserData for Element {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_function(
            MetaMethod::Index,
            |lua, (this, key): (AnyUserData, mlua::String)| {
                let method = key.to_str()?.to_owned();
                let cache: Table = lua.named_registry_value("gpui_shell.methods")?;

                let cached = cache.get::<LuaValue>(method.as_str())?;
                if matches!(cached, LuaValue::Function(_)) {
                    return Ok(cached);
                }

                // Reject unknown names at lookup time so a typo reports at the
                // call site with a suggestion, the way a compiler would.
                if !is_known_method(&method) {
                    let _ = this;
                    return Err(LuaError::runtime(unknown_method_message(&method)));
                }

                let name = method.clone();
                let function =
                    lua.create_function(move |_, (element, args): (AnyUserData, MultiValue)| {
                        if name == "when" {
                            return apply_when(element, args);
                        }
                        let (id, runtime) = {
                            let element = element.borrow::<Element>()?;
                            (element.id, upgrade(&element.runtime)?)
                        };
                        runtime.apply(id, &name, args)?;
                        Ok(LuaValue::UserData(element))
                    })?;
                cache.set(method, &function)?;
                Ok(LuaValue::Function(function))
            },
        );
    }
}

/// The Lua-side `cx`. It stores only a generation; every use is checked against
/// the live scope stack.
#[derive(Clone, Copy)]
pub struct LuaContext {
    generation: u64,
}

impl UserData for LuaContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("notify", |_, this, ()| {
            let phase = scope::current_phase();
            if !phase.is_some_and(ScopePhase::allows_notify) {
                return Err(LuaError::runtime(format!(
                    "cx:notify() is not allowed during the `{}` phase; \
                     request a re-render from an event handler instead",
                    phase.map(ScopePhase::as_str).unwrap_or("none")
                )));
            }

            let view = scope::current_view();
            scope::with_context(this.generation, move |_, app| {
                if let Some(view) = view {
                    view.update(app, |_, cx| cx.notify());
                }
            })
            .map_err(|error| LuaError::runtime(error.to_string()))
        });

        methods.add_method("phase", |_, this, ()| {
            let _ = this;
            Ok(scope::current_phase()
                .map(ScopePhase::as_str)
                .unwrap_or("none"))
        });
    }
}

/// Conditional refinement, mirroring GPUI's `FluentBuilder::when`. It keeps a
/// builder chain in one piece instead of forcing a mutable temporary.
fn apply_when(element: AnyUserData, args: MultiValue) -> LuaResult<LuaValue> {
    let mut arguments = args.into_iter();
    let condition = arguments.next().unwrap_or(LuaValue::Nil);
    let branch = arguments.next().unwrap_or(LuaValue::Nil);

    let LuaValue::Function(branch) = branch else {
        return Err(LuaError::runtime(
            "`when` expects (condition, function(el) ... end)".to_string(),
        ));
    };

    let taken = !matches!(condition, LuaValue::Nil | LuaValue::Boolean(false));
    if !taken {
        return Ok(LuaValue::UserData(element));
    }

    let produced: LuaValue = branch.call(element)?;
    element_id(&produced).map(|_| produced)
}

fn is_known_method(name: &str) -> bool {
    matches!(
        name,
        "child"
            | "children"
            | "on_click"
            | "on_change"
            | "disabled"
            | "selected"
            | "checked"
            | "when"
    ) || style::nullary_index(name).is_some()
        || style::param_style_name(name).is_some()
}

fn unknown_method_message(name: &str) -> String {
    match style::suggest(name) {
        Some(candidate) => {
            format!("unknown element method `{name}` (did you mean `{candidate}`?)")
        }
        None => format!(
            "unknown element method `{name}`; it is neither a style method nor one of \
             child, children, on_click, on_change, disabled, selected, checked, when"
        ),
    }
}

fn upgrade(runtime: &Weak<ShellRuntime>) -> LuaResult<Rc<ShellRuntime>> {
    runtime
        .upgrade()
        .ok_or_else(|| LuaError::runtime("the shell runtime has already shut down"))
}

fn element_id(value: &LuaValue) -> LuaResult<SpecId> {
    match value {
        LuaValue::UserData(data) => Ok(data.borrow::<Element>()?.id),
        other => Err(LuaError::runtime(format!(
            "expected an element, got {}",
            other.type_name()
        ))),
    }
}

fn element_argument(args: &MultiValue, index: usize, method: &str) -> LuaResult<SpecId> {
    let value = args
        .iter()
        .nth(index)
        .ok_or_else(|| LuaError::runtime(format!("`{method}` expects an element argument")))?;
    element_id(value)
}

fn bridge_args(args: &MultiValue) -> LuaResult<SmallVec<[Bridged; 2]>> {
    args.iter().map(bridge_value).collect()
}

/// The engine half of value bridging. `Bridged` itself stays engine neutral.
fn bridge_value(value: &LuaValue) -> LuaResult<Bridged> {
    Ok(match value {
        LuaValue::Nil => Bridged::Nil,
        LuaValue::Boolean(flag) => Bridged::Bool(*flag),
        LuaValue::Integer(number) => Bridged::Number(*number as f64),
        LuaValue::Number(number) => Bridged::Number(*number),
        LuaValue::String(text) => Bridged::Str(text.to_str()?.to_owned()),
        other => {
            return Err(LuaError::runtime(format!(
                "unsupported argument type `{}`; expected nil, boolean, number or string",
                other.type_name()
            )));
        }
    })
}

fn click_event_table(lua: &Lua, event: &ClickEvent) -> LuaResult<Table> {
    let table = lua.create_table()?;
    table.set("click_count", event.click_count())?;

    let modifiers = lua.create_table()?;
    let flags = event.modifiers();
    modifiers.set("shift", flags.shift)?;
    modifiers.set("control", flags.control)?;
    modifiers.set("alt", flags.alt)?;
    modifiers.set("platform", flags.platform)?;
    table.set("modifiers", modifiers)?;

    Ok(table)
}

/// Registers the method cache table. Called once per VM.
pub fn install_method_cache(lua: &Lua) -> LuaResult<()> {
    let cache = lua.create_table()?;
    lua.set_named_registry_value("gpui_shell.methods", cache)
}
