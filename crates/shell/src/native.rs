//! Native modules — Rust functions the host lends to a script.
//!
//! A script cannot load a native extension. `dlopen`-ed Rust has no stable ABI
//! and, once inside the process, holds every permission the process holds — a
//! sandbox that permits it does not mean anything. So the direction is
//! reversed: **the host registers, at compile time, the Rust it is willing to
//! expose**, and a script may reach exactly that and nothing else (design doc
//! §17.6).
//!
//! ```no_run
//! use gpui_shell::{NativeModules, NativeValue};
//!
//! let mut modules = NativeModules::new();
//! modules.register("workspace", |module| {
//!     module.function("project_name", |_| Ok(NativeValue::from("gpui-component")));
//! });
//! gpui_shell::set_native_modules(modules);
//! ```
//!
//! ```js
//! import { native } from "gpui";
//!
//! const workspace = native("workspace");
//! workspace.project_name();
//! ```
//!
//! # Why the boundary is plain data
//!
//! A native function receives [`NativeArguments`] and returns a
//! [`NativeValue`]: null, boolean, number, string, array, object. It never
//! receives a script handle. That is not a convenience — a handle would let the
//! host keep a reference to a script value past the call that produced it, and
//! past the [`crate::scope`] frame that made the surrounding context valid. It
//! is also what lets one registry serve both engines, since neither engine's
//! value type appears in this file.
//!
//! # Why a native function must not re-enter the engine
//!
//! A native call happens *inside* a script call, which is itself inside a host
//! call. Calling back into the VM from there would run script code with an
//! engine frame already on the stack — re-entering QuickJS, and re-entering the
//! render pass that is currently building an element tree. Holding no script
//! handle makes that impossible to express, and [`dispatch`] refuses a nested
//! call outright so a host that finds another route (pumping GPUI until a view
//! re-renders, say) gets a diagnosable error instead of undefined behavior.
//!
//! Reading and writing host state is fine, and is the point: a function may
//! reach for the ambient `App` through [`crate::scope::with_current_app`] and
//! request a re-render with `cx.notify()`, which is delivered after the current
//! call unwinds.
//!
//! # Reaching native modules is itself a capability
//!
//! The default registry is empty, and every entry point into it fails while it
//! stays that way — the same shape as [`crate::Capabilities::default`], which
//! permits nothing. A host that installs no modules has granted no native
//! access, and a script that asks for one is told so by name. There is
//! deliberately no per-module grant: the host chose the module list, so the
//! list *is* the grant.

use std::{cell::Cell, collections::BTreeMap, fmt, rc::Rc};

/// A value crossing the native boundary, in either direction.
///
/// The six cases are the intersection of what a script engine and JSON can both
/// carry, which is what keeps one registry usable from any engine behind the
/// seam rather than from QuickJS alone.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<NativeValue>),
    /// Insertion-ordered, because an object is frequently a record the script
    /// renders in order, and a map would decide that order for it.
    Object(Vec<(String, NativeValue)>),
}

impl NativeValue {
    pub fn is_null(&self) -> bool {
        matches!(self, NativeValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            NativeValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            NativeValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            NativeValue::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[NativeValue]> {
        match self {
            NativeValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, NativeValue)]> {
        match self {
            NativeValue::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// The value of one object field, or `None` for a non-object.
    pub fn get(&self, key: &str) -> Option<&NativeValue> {
        self.as_object()?
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The type name used in error messages.
    pub fn describe(&self) -> &'static str {
        match self {
            NativeValue::Null => "null",
            NativeValue::Bool(_) => "a boolean",
            NativeValue::Number(_) => "a number",
            NativeValue::Str(_) => "a string",
            NativeValue::Array(_) => "an array",
            NativeValue::Object(_) => "an object",
        }
    }
}

impl From<bool> for NativeValue {
    fn from(value: bool) -> Self {
        NativeValue::Bool(value)
    }
}

macro_rules! from_number {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for NativeValue {
                fn from(value: $type) -> Self {
                    NativeValue::Number(value as f64)
                }
            }
        )*
    };
}

from_number!(f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<String> for NativeValue {
    fn from(value: String) -> Self {
        NativeValue::Str(value)
    }
}

impl From<&str> for NativeValue {
    fn from(value: &str) -> Self {
        NativeValue::Str(value.to_owned())
    }
}

impl<T: Into<NativeValue>> From<Vec<T>> for NativeValue {
    fn from(values: Vec<T>) -> Self {
        NativeValue::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<NativeValue>> From<Option<T>> for NativeValue {
    fn from(value: Option<T>) -> Self {
        value.map(Into::into).unwrap_or(NativeValue::Null)
    }
}

/// Builds a [`NativeValue::Object`] one field at a time.
///
/// A record is the common return shape — a row, a settings snapshot, a
/// progress report — and building it with a builder keeps the field order the
/// host wrote it in.
#[derive(Clone, Debug, Default)]
pub struct NativeObject {
    fields: Vec<(String, NativeValue)>,
}

impl NativeObject {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field. A repeated name replaces the earlier value in place, so a
    /// caller cannot accidentally emit the same key twice.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<NativeValue>) -> Self {
        let name = name.into();
        let value = value.into();
        match self.fields.iter_mut().find(|(key, _)| *key == name) {
            Some(existing) => existing.1 = value,
            None => self.fields.push((name, value)),
        }
        self
    }
}

impl From<NativeObject> for NativeValue {
    fn from(object: NativeObject) -> Self {
        NativeValue::Object(object.fields)
    }
}

/// The positional arguments of one native call.
///
/// The typed readers exist so that a wrong argument reports which position was
/// wrong and what arrived there, rather than the host writing that sentence
/// once per function.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeArguments {
    values: Vec<NativeValue>,
}

impl NativeArguments {
    pub fn new(values: impl IntoIterator<Item = NativeValue>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&NativeValue> {
        self.values.get(index)
    }

    /// The argument at `index`, or an error naming the position.
    pub fn value(&self, index: usize) -> Result<&NativeValue, NativeError> {
        self.values.get(index).ok_or_else(|| {
            NativeError::new(format!(
                "argument {} is missing; {} were passed",
                index + 1,
                self.values.len()
            ))
        })
    }

    pub fn string(&self, index: usize) -> Result<&str, NativeError> {
        let value = self.value(index)?;
        value
            .as_str()
            .ok_or_else(|| mistyped(index, "a string", value))
    }

    pub fn number(&self, index: usize) -> Result<f64, NativeError> {
        let value = self.value(index)?;
        value
            .as_number()
            .ok_or_else(|| mistyped(index, "a number", value))
    }

    /// A number that has to be whole — an identifier, a count, an index.
    pub fn integer(&self, index: usize) -> Result<i64, NativeError> {
        let number = self.number(index)?;
        if number.fract() != 0. {
            return Err(NativeError::new(format!(
                "argument {} must be a whole number, got {number}",
                index + 1
            )));
        }
        Ok(number as i64)
    }

    pub fn boolean(&self, index: usize) -> Result<bool, NativeError> {
        let value = self.value(index)?;
        value
            .as_bool()
            .ok_or_else(|| mistyped(index, "a boolean", value))
    }
}

fn mistyped(index: usize, expected: &str, got: &NativeValue) -> NativeError {
    NativeError::new(format!(
        "argument {} must be {expected}, got {}",
        index + 1,
        got.describe()
    ))
}

/// A native function said no.
///
/// It carries a sentence and nothing else: the engine adds the module and
/// function names when it turns this into a script exception, so a host writing
/// a function never repeats its own name in the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeError {
    message: String,
}

impl NativeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NativeError {}

impl From<String> for NativeError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for NativeError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// What a native function returns.
pub type NativeResult = Result<NativeValue, NativeError>;

/// `Rc` rather than `Box` because the registry is handed out by clone on every
/// call — see [`modules`] — and a boxed closure cannot be shared that way.
type NativeFunction = Rc<dyn Fn(&NativeArguments) -> NativeResult>;

/// One registered module: a name and the functions under it.
pub struct NativeModule {
    name: String,
    /// Sorted, so the "it provides: …" line in a diagnostic reads the same on
    /// every run regardless of registration order.
    functions: BTreeMap<String, NativeFunction>,
}

impl NativeModule {
    fn new(name: String) -> Self {
        Self {
            name,
            functions: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Registers one function.
    ///
    /// The body must not call back into the script engine; see the module
    /// header. It may read and write host state, and may ask a view to
    /// re-render — the notification is delivered after the call unwinds.
    pub fn function(
        &mut self,
        name: impl Into<String>,
        body: impl Fn(&NativeArguments) -> NativeResult + 'static,
    ) -> &mut Self {
        self.functions.insert(name.into(), Rc::new(body));
        self
    }

    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(String::as_str).collect()
    }

    pub fn has(&self, function: &str) -> bool {
        self.functions.contains_key(function)
    }

    /// Calls one function, reporting an unknown name against what this module
    /// actually provides.
    pub fn call(&self, function: &str, arguments: &NativeArguments) -> NativeResult {
        let Some(body) = self.functions.get(function) else {
            return Err(NativeError::new(format!(
                "native module `{}` has no function `{function}`; it provides: {}",
                self.name,
                list(&self.function_names())
            )));
        };
        body(arguments)
    }
}

/// Every native module a host has granted.
///
/// Empty by default, which denies everything.
#[derive(Default)]
pub struct NativeModules {
    modules: BTreeMap<String, NativeModule>,
}

impl NativeModules {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a module, building its functions through `build`.
    ///
    /// Registering the same name twice replaces the earlier module rather than
    /// merging into it: two registrations of one name are a mistake, and
    /// merging would hide it behind a module that half works.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        build: impl FnOnce(&mut NativeModule),
    ) -> &mut Self {
        let name = name.into();
        let mut module = NativeModule::new(name.clone());
        build(&mut module);
        self.modules.insert(name, module);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn module_names(&self) -> Vec<&str> {
        self.modules.keys().map(String::as_str).collect()
    }

    /// Looks a module up, reporting a miss against the granted set.
    ///
    /// The two failures are different facts and get different sentences: a host
    /// that granted nothing is a host that has not wired native access up, and
    /// telling that author "unknown module" would send them hunting for a typo
    /// that is not there.
    pub fn get(&self, name: &str) -> Result<&NativeModule, NativeError> {
        if let Some(module) = self.modules.get(name) {
            return Ok(module);
        }

        Err(NativeError::new(if self.modules.is_empty() {
            format!(
                "native module `{name}` is not available: this host registered none. \
                 Native modules are granted by the embedding application, with \
                 gpui_shell::set_native_modules(...)."
            )
        } else {
            format!(
                "unknown native module `{name}`; this host registered: {}",
                list(&self.module_names())
            )
        }))
    }

    /// Resolves and calls in one step, for a host driving the registry
    /// directly. The engine goes through [`dispatch`] instead.
    pub fn call(&self, module: &str, function: &str, arguments: &NativeArguments) -> NativeResult {
        self.get(module)?.call(function, arguments)
    }
}

fn list(names: &[&str]) -> String {
    if names.is_empty() {
        "nothing".to_owned()
    } else {
        names.join(", ")
    }
}

thread_local! {
    /// The installed registry.
    ///
    /// Thread-local for the same reason the capability grant is: the VM and
    /// Depth guard for [`dispatch`].
    static IN_CALL: Cell<bool> = const { Cell::new(false) };
}

/// Installs the modules a script may reach, replacing any previous set.
///
/// May be called at any point before the script calls `native(...)`; the
/// registry is read at call time, so revoking a module takes effect on the next
/// call rather than on the next restart.
pub(crate) fn set_modules(modules: NativeModules) {
    crate::policy::update_default(|policy| policy.with_native_modules(modules));
}

/// Removes every installed module.
///
/// A host closure typically captures a GPUI entity handle — that is how a native
/// function reaches host state at all — so the registry keeps those handles
/// alive for as long as it holds the closure. A host that goes away without
/// clearing leaves them registered, which GPUI reports as a leaked handle at
/// shutdown and which would be a real leak for a plugin host that unloads and
/// reloads.
///
/// So clearing is the installer's job, in the same place it would drop anything
/// else it owns.
pub(crate) fn clear_modules() {
    set_modules(NativeModules::default());
}

/// The registry the code now running may reach.
///
/// Read through the calling frame, so a plugin sees the modules its own host
/// registered for it rather than whichever set was installed most recently.
pub(crate) fn modules() -> Rc<NativeModules> {
    crate::scope::policy().modules()
}

/// The one path from an engine into host code.
///
/// Refuses a nested call: a native function that has found a way to run script
/// code — and so to reach a second native function — has re-entered the engine,
/// which the module header explains is not allowed. Reporting it is the whole
/// value here; the alternative is a re-entrant render pass that fails somewhere
/// else entirely.
pub(crate) fn dispatch(module: &str, function: &str, arguments: &NativeArguments) -> NativeResult {
    if IN_CALL.with(Cell::get) {
        return Err(NativeError::new(format!(
            "`{module}.{function}` was reached from inside another native call: \
             a native function may not call back into the script engine"
        )));
    }

    let registry = modules();
    let _guard = CallGuard::enter();
    registry.call(module, function, arguments)
}

/// Clears the depth guard however the call leaves — returned, failed, or
/// unwound. A flag left set would deny every later call.
struct CallGuard;

impl CallGuard {
    fn enter() -> Self {
        IN_CALL.with(|flag| flag.set(true));
        Self
    }
}

impl Drop for CallGuard {
    fn drop(&mut self) {
        IN_CALL.with(|flag| flag.set(false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> NativeModules {
        let mut modules = NativeModules::new();
        modules.register("workspace", |module| {
            module.function("project_name", |_| Ok(NativeValue::from("gpui-component")));
            module.function("open_count", |arguments| {
                Ok(NativeValue::from(arguments.len()))
            });
            module.function("echo", |arguments| Ok(arguments.value(0)?.clone()));
            module.function("close", |arguments| {
                let id = arguments.integer(0)?;
                Err(NativeError::new(format!("tab {id} is already closed")))
            });
        });
        modules.register("editor", |module| {
            module.function("line_count", |_| Ok(NativeValue::from(12)));
        });
        modules
    }

    #[test]
    fn a_registered_function_is_callable_and_returns_its_value() {
        let modules = registry();
        assert_eq!(
            modules
                .call("workspace", "project_name", &NativeArguments::default())
                .unwrap(),
            NativeValue::Str("gpui-component".into())
        );
        assert_eq!(
            modules
                .call(
                    "workspace",
                    "open_count",
                    &NativeArguments::new([NativeValue::from(1), NativeValue::from(2)])
                )
                .unwrap(),
            NativeValue::Number(2.)
        );
    }

    #[test]
    fn an_unregistered_module_reports_the_registered_ones() {
        let error = registry()
            .call("workspce", "project_name", &NativeArguments::default())
            .unwrap_err();
        assert_eq!(
            error.message(),
            "unknown native module `workspce`; this host registered: editor, workspace"
        );
    }

    #[test]
    fn an_empty_registry_says_the_host_granted_nothing() {
        let error = NativeModules::new()
            .call("workspace", "project_name", &NativeArguments::default())
            .unwrap_err();
        assert!(error.message().contains("this host registered none"));
        assert!(error.message().contains("set_native_modules"));
    }

    #[test]
    fn an_unknown_function_reports_what_the_module_provides() {
        let error = registry()
            .call("editor", "line_cont", &NativeArguments::default())
            .unwrap_err();
        assert_eq!(
            error.message(),
            "native module `editor` has no function `line_cont`; it provides: line_count"
        );
    }

    #[test]
    fn a_failing_function_surfaces_its_message() {
        let error = registry()
            .call(
                "workspace",
                "close",
                &NativeArguments::new([NativeValue::from(7)]),
            )
            .unwrap_err();
        assert_eq!(error.message(), "tab 7 is already closed");
    }

    #[test]
    fn a_mistyped_argument_names_its_position_and_what_arrived() {
        let error = registry()
            .call(
                "workspace",
                "close",
                &NativeArguments::new([NativeValue::from("seven")]),
            )
            .unwrap_err();
        assert_eq!(error.message(), "argument 1 must be a number, got a string");
    }

    #[test]
    fn a_nested_value_round_trips_through_the_boundary_type() {
        let value: NativeValue = NativeObject::new()
            .field("name", "release")
            .field("done", true)
            .field("progress", 0.5)
            .field("owner", None::<String>)
            .field(
                "steps",
                vec![
                    NativeValue::from(NativeObject::new().field("id", 1).field("title", "Tag")),
                    NativeValue::from(NativeObject::new().field("id", 2).field("title", "Ship")),
                ],
            )
            .into();

        let returned = registry()
            .call("workspace", "echo", &NativeArguments::new([value.clone()]))
            .unwrap();

        assert_eq!(returned, value);
        assert_eq!(
            returned.get("name").and_then(NativeValue::as_str),
            Some("release")
        );
        assert!(returned.get("owner").is_some_and(NativeValue::is_null));
        assert_eq!(
            returned
                .get("steps")
                .and_then(NativeValue::as_array)
                .and_then(|steps| steps[1].get("title"))
                .and_then(NativeValue::as_str),
            Some("Ship")
        );
    }

    #[test]
    fn a_repeated_field_replaces_the_earlier_value_in_place() {
        let value: NativeValue = NativeObject::new()
            .field("id", 1)
            .field("title", "Tag")
            .field("id", 2)
            .into();

        assert_eq!(
            value,
            NativeValue::Object(vec![
                ("id".into(), NativeValue::Number(2.)),
                ("title".into(), NativeValue::Str("Tag".into())),
            ])
        );
    }

    #[test]
    fn registering_a_name_twice_replaces_the_module() {
        let mut modules = NativeModules::new();
        modules.register("workspace", |module| {
            module.function("first", |_| Ok(NativeValue::Null));
        });
        modules.register("workspace", |module| {
            module.function("second", |_| Ok(NativeValue::Null));
        });

        assert_eq!(
            modules.get("workspace").unwrap().function_names(),
            vec!["second"]
        );
    }

    #[test]
    fn the_installed_registry_is_what_dispatch_calls() {
        set_modules(registry());

        assert_eq!(
            dispatch("editor", "line_count", &NativeArguments::default()).unwrap(),
            NativeValue::Number(12.)
        );

        set_modules(NativeModules::new());
        assert!(modules().is_empty());
    }

    #[test]
    fn a_native_function_cannot_reach_a_second_one() {
        let mut modules = NativeModules::new();
        modules.register("loop", |module| {
            module.function("outer", |_| {
                dispatch("loop", "inner", &NativeArguments::default())
            });
            module.function("inner", |_| Ok(NativeValue::Null));
        });
        set_modules(modules);

        let error = dispatch("loop", "outer", &NativeArguments::default()).unwrap_err();
        assert!(
            error
                .message()
                .contains("may not call back into the script engine"),
            "{}",
            error.message()
        );

        // The guard is released even after a refusal, so the next call works.
        assert_eq!(
            dispatch("loop", "inner", &NativeArguments::default()).unwrap(),
            NativeValue::Null
        );

        set_modules(NativeModules::new());
    }
}
