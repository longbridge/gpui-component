//! Host modules — Rust functions the host lends to a script, imported by name.
//!
//! A script cannot load a native extension. `dlopen`-ed Rust has no stable ABI
//! and, once inside the process, holds every permission the process holds — a
//! sandbox that permits it does not mean anything. So the direction is
//! reversed: **the host registers, at compile time, the Rust it is willing to
//! expose**, and a script may reach exactly that and nothing else (design doc
//! §17.6).
//!
//! ```no_run
//! use gpui_shell::{HostModules, HostValue};
//!
//! let mut modules = HostModules::new();
//! modules.register("workspace", |module| {
//!     module.function("project_name", |_| Ok(HostValue::from("gpui-component")));
//! });
//! gpui_shell::export_modules(modules).expect("the module names are free");
//! ```
//!
//! ```js
//! import { project_name } from "workspace";
//!
//! project_name();
//! ```
//!
//! # Why an import rather than a lookup
//!
//! A registered module is resolved by the engine's module loader, so a host
//! module is imported exactly like `gpui` or `path` is. The alternative — a
//! `native("workspace")` call answering with a bag of functions — put every
//! misspelling on the run-time path, and left the type declarations unable to
//! say anything: only the host knows what it registered, so the best a
//! generated `.d.ts` could offer was `Record<string, (...args) => any>`.
//!
//! As an import, a wrong export name fails while the module graph is linked,
//! before a line of the application runs, and [`crate::typings`] can emit the
//! names from this registry directly.
//!
//! What the import does *not* freeze is the function behind the name. Every
//! export is a forwarding stub that resolves through [`dispatch`] on each call,
//! so revoking a module still takes effect immediately — a script holding an
//! imported function gets a refusal rather than the withdrawn closure. Only the
//! *set of names* is fixed, at the moment the importing module is linked, which
//! is why a host registers before it loads an application.
//!
//! # Why the boundary is plain data
//!
//! A host function receives [`HostArguments`] and returns a [`HostValue`]:
//! null, boolean, number, string, array, object. It never receives a script
//! handle. That is not a convenience — a handle would let the host keep a
//! reference to a script value past the call that produced it, and past the
//! internal call-scope frame that made the surrounding context valid. It is
//! also what lets one registry serve both engines, since neither engine's value
//! type appears in this file.
//!
//! # Why a host function must not re-enter the engine
//!
//! A host call happens *inside* a script call, which is itself inside a host
//! call. Calling back into the VM from there would run script code with an
//! engine frame already on the stack — re-entering QuickJS, and re-entering the
//! render pass that is currently building an element tree. Holding no script
//! handle makes that impossible to express, and the internal dispatcher refuses a nested
//! call outright so a host that finds another route (pumping GPUI until a view
//! re-renders, say) gets a diagnosable error instead of undefined behavior.
//!
//! Reading and writing host state is fine, and is the point: a function may
//! reach for the ambient `App` through [`crate::scope::with_current_app`] and
//! request a re-render with `cx.notify()`, which is delivered after the current
//! call unwinds.
//!
//! # Reaching host modules is itself a capability
//!
//! The default registry is empty, and every entry point into it fails while it
//! stays that way — the same shape as [`crate::Capabilities::default`], which
//! permits nothing. A host that installs no modules has granted no extension
//! surface, and a script that imports one is told so by name. There is
//! deliberately no per-module grant: the host chose the module list, so the
//! list *is* the grant.
//!
//! # Why the runtime's own modules cannot be taken
//!
//! A host module shares one specifier namespace with the built-in modules and
//! the Standard Runtime, and the resolver chain reaches the built-ins first. A
//! host registering `path` would therefore not shadow the real `path` — it
//! would register a module nothing can ever import, and never find out. So the
//! names in [`RESERVED_SPECIFIERS`] are refused at registration, where the
//! author can still see the sentence.

use std::{cell::Cell, collections::BTreeMap, fmt, fmt::Write as _, rc::Rc};

/// A value crossing the host boundary, in either direction.
///
/// The six cases are the intersection of what a script engine and JSON can both
/// carry, which is what keeps one registry usable from any engine behind the
/// seam rather than from QuickJS alone.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<HostValue>),
    /// Insertion-ordered, because an object is frequently a record the script
    /// renders in order, and a map would decide that order for it.
    Object(Vec<(String, HostValue)>),
}

impl HostValue {
    pub fn is_null(&self) -> bool {
        matches!(self, HostValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            HostValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            HostValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            HostValue::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[HostValue]> {
        match self {
            HostValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, HostValue)]> {
        match self {
            HostValue::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// The value of one object field, or `None` for a non-object.
    pub fn get(&self, key: &str) -> Option<&HostValue> {
        self.as_object()?
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The type name used in error messages.
    pub fn describe(&self) -> &'static str {
        match self {
            HostValue::Null => "null",
            HostValue::Bool(_) => "a boolean",
            HostValue::Number(_) => "a number",
            HostValue::Str(_) => "a string",
            HostValue::Array(_) => "an array",
            HostValue::Object(_) => "an object",
        }
    }
}

impl From<bool> for HostValue {
    fn from(value: bool) -> Self {
        HostValue::Bool(value)
    }
}

macro_rules! from_number {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for HostValue {
                fn from(value: $type) -> Self {
                    HostValue::Number(value as f64)
                }
            }
        )*
    };
}

from_number!(f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<String> for HostValue {
    fn from(value: String) -> Self {
        HostValue::Str(value)
    }
}

impl From<&str> for HostValue {
    fn from(value: &str) -> Self {
        HostValue::Str(value.to_owned())
    }
}

impl<T: Into<HostValue>> From<Vec<T>> for HostValue {
    fn from(values: Vec<T>) -> Self {
        HostValue::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<HostValue>> From<Option<T>> for HostValue {
    fn from(value: Option<T>) -> Self {
        value.map(Into::into).unwrap_or(HostValue::Null)
    }
}

/// Builds a [`HostValue::Object`] one field at a time.
///
/// A record is the common return shape — a row, a settings snapshot, a
/// progress report — and building it with a builder keeps the field order the
/// host wrote it in.
#[derive(Clone, Debug, Default)]
pub struct HostObject {
    fields: Vec<(String, HostValue)>,
}

impl HostObject {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field. A repeated name replaces the earlier value in place, so a
    /// caller cannot accidentally emit the same key twice.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<HostValue>) -> Self {
        let name = name.into();
        let value = value.into();
        match self.fields.iter_mut().find(|(key, _)| *key == name) {
            Some(existing) => existing.1 = value,
            None => self.fields.push((name, value)),
        }
        self
    }
}

impl From<HostObject> for HostValue {
    fn from(object: HostObject) -> Self {
        HostValue::Object(object.fields)
    }
}

/// The positional arguments of one host call.
///
/// The typed readers exist so that a wrong argument reports which position was
/// wrong and what arrived there, rather than the host writing that sentence
/// once per function.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HostArguments {
    values: Vec<HostValue>,
}

impl HostArguments {
    pub fn new(values: impl IntoIterator<Item = HostValue>) -> Self {
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

    pub fn get(&self, index: usize) -> Option<&HostValue> {
        self.values.get(index)
    }

    /// The argument at `index`, or an error naming the position.
    pub fn value(&self, index: usize) -> Result<&HostValue, HostError> {
        self.values.get(index).ok_or_else(|| {
            HostError::new(format!(
                "argument {} is missing; {} were passed",
                index + 1,
                self.values.len()
            ))
        })
    }

    pub fn string(&self, index: usize) -> Result<&str, HostError> {
        let value = self.value(index)?;
        value
            .as_str()
            .ok_or_else(|| mistyped(index, "a string", value))
    }

    pub fn number(&self, index: usize) -> Result<f64, HostError> {
        let value = self.value(index)?;
        value
            .as_number()
            .ok_or_else(|| mistyped(index, "a number", value))
    }

    /// A number that has to be whole — an identifier, a count, an index.
    pub fn integer(&self, index: usize) -> Result<i64, HostError> {
        let number = self.number(index)?;
        if number.fract() != 0. {
            return Err(HostError::new(format!(
                "argument {} must be a whole number, got {number}",
                index + 1
            )));
        }
        Ok(number as i64)
    }

    pub fn boolean(&self, index: usize) -> Result<bool, HostError> {
        let value = self.value(index)?;
        value
            .as_bool()
            .ok_or_else(|| mistyped(index, "a boolean", value))
    }
}

fn mistyped(index: usize, expected: &str, got: &HostValue) -> HostError {
    HostError::new(format!(
        "argument {} must be {expected}, got {}",
        index + 1,
        got.describe()
    ))
}

/// A host function said no.
///
/// It carries a sentence and nothing else: the engine adds the module and
/// function names when it turns this into a script exception, so a host writing
/// a function never repeats its own name in the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HostError {}

impl From<String> for HostError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for HostError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// What a host function returns.
pub type HostResult = Result<HostValue, HostError>;

/// `Rc` rather than `Box` because the registry is handed out by clone on every
/// call — see [`modules`] — and a boxed closure cannot be shared that way.
type HostFunction = Rc<dyn Fn(&HostArguments) -> HostResult>;

/// One registered module: a name and the functions under it.
pub struct HostModule {
    name: String,
    /// Sorted, so the "it provides: …" line in a diagnostic reads the same on
    /// every run regardless of registration order.
    functions: BTreeMap<String, HostFunction>,
    /// The module's TypeScript face, if the host wrote one. See
    /// [`HostModule::declarations`].
    declarations: Option<String>,
}

impl HostModule {
    fn new(name: String) -> Self {
        Self {
            name,
            functions: BTreeMap::new(),
            declarations: None,
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
        body: impl Fn(&HostArguments) -> HostResult + 'static,
    ) -> &mut Self {
        self.functions.insert(name.into(), Rc::new(body));
        self
    }

    /// Describes this module in TypeScript, for the generated declarations.
    ///
    /// The body of a `declare module`, so it may carry helper types alongside
    /// the exports:
    ///
    /// ```no_run
    /// # use gpui_shell::{HostModules, HostValue};
    /// # let mut modules = HostModules::new();
    /// modules.register("market", |module| {
    ///     module.function("quotes", |_| Ok(HostValue::Null));
    ///     module.declarations(
    ///         r#"
    ///         export interface Quote { symbol: string; last: string }
    ///         export function quotes(): Quote[];
    ///         "#,
    ///     );
    /// });
    /// ```
    ///
    /// Writing it here rather than in a `.d.ts` beside the script is what makes
    /// the two halves one thing. They used to be two files in two languages
    /// with nothing between them, and [`HostModules::validate`] now checks that
    /// every registered function is declared and every declared function is
    /// registered — so renaming one half fails at start-up instead of at the
    /// call site.
    ///
    /// Declaring nothing is allowed and costs only precision: an undeclared
    /// module is emitted with `(...args: any[]) => any` signatures, which still
    /// checks the module name and every export name.
    pub fn declarations(&mut self, typescript: impl Into<String>) -> &mut Self {
        self.declarations = Some(typescript.into());
        self
    }

    /// The TypeScript body [`Self::declarations`] was given.
    pub fn declared(&self) -> Option<&str> {
        self.declarations.as_deref()
    }

    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(String::as_str).collect()
    }

    pub fn has(&self, function: &str) -> bool {
        self.functions.contains_key(function)
    }

    /// Compares the declared exports with the registered ones.
    ///
    /// Only `export function` and `export const` at the start of a line are
    /// read as exports — enough to name every function a module can have, and
    /// deliberately not a TypeScript parser: anything it cannot recognize it
    /// leaves alone, so a helper type or a comment is never mistaken for a
    /// missing function.
    fn check_declarations(&self) -> Result<(), HostError> {
        let Some(declarations) = self.declared() else {
            return Ok(());
        };

        let declared: Vec<&str> = declarations
            .lines()
            .filter_map(|line| {
                let line = line.trim_start();
                let rest = line
                    .strip_prefix("export function ")
                    .or_else(|| line.strip_prefix("export declare function "))
                    .or_else(|| line.strip_prefix("export const "))?;
                let end =
                    rest.find(|it: char| !it.is_ascii_alphanumeric() && it != '_' && it != '$');
                Some(&rest[..end.unwrap_or(rest.len())])
            })
            .collect();

        let missing: Vec<&str> = self
            .function_names()
            .into_iter()
            .filter(|name| !declared.contains(name))
            .collect();
        let extra: Vec<&str> = declared
            .iter()
            .copied()
            .filter(|name| !self.has(name))
            .collect();

        if missing.is_empty() && extra.is_empty() {
            return Ok(());
        }

        let mut message = format!(
            "host module `{}` declares a different set of functions than it registers",
            self.name
        );
        if !missing.is_empty() {
            let _ = write!(message, "; registered but not declared: {}", list(&missing));
        }
        if !extra.is_empty() {
            let _ = write!(message, "; declared but not registered: {}", list(&extra));
        }
        Err(HostError::new(message))
    }

    /// Calls one function, reporting an unknown name against what this module
    /// actually provides.
    pub fn call(&self, function: &str, arguments: &HostArguments) -> HostResult {
        let Some(body) = self.functions.get(function) else {
            return Err(HostError::new(format!(
                "host module `{}` has no function `{function}`; it provides: {}",
                self.name,
                list(&self.function_names())
            )));
        };
        body(arguments)
    }
}

/// The specifiers a host may not register, because something else answers them.
///
/// Two groups, and the reason is the same for both: the resolver chain reaches
/// them before it reaches this registry, so a host module wearing one of these
/// names is unreachable rather than overriding. The engine asserts this list
/// against its own resolvers, so a module added there and not here is a failing
/// test rather than a name a host can quietly lose.
pub const RESERVED_SPECIFIERS: &[&str] = &[
    // The runtime's own modules.
    "gpui",
    "gpui-base",
    "gpui-fps",
    // The Standard Runtime.
    "buffer",
    "console",
    "crypto",
    "fs/promises",
    "net",
    "os",
    "path",
    "process",
    "url",
    "websocket",
    "zlib",
];

/// Allocates the identity of one registry. See [`HostModules::generation`].
fn next_generation() -> u64 {
    thread_local! {
        static NEXT: Cell<u64> = const { Cell::new(1) };
    }
    NEXT.with(|next| {
        let generation = next.get();
        next.set(generation.wrapping_add(1));
        generation
    })
}

/// Every host module a host has granted.
///
/// Empty by default, which denies everything.
pub struct HostModules {
    modules: BTreeMap<String, HostModule>,
    /// Names that were refused, reported together by [`Self::validate`].
    rejected: Vec<String>,
    generation: u64,
}

impl Default for HostModules {
    fn default() -> Self {
        Self {
            modules: BTreeMap::new(),
            rejected: Vec::new(),
            generation: next_generation(),
        }
    }
}

impl HostModules {
    pub fn new() -> Self {
        Self::default()
    }

    /// What distinguishes this registry from every other one built on this
    /// thread.
    ///
    /// The engine caches a linked module by its resolved name, for the lifetime
    /// of the runtime — so two plugins importing `workspace`, or one host
    /// re-registering after a change, would otherwise share whichever module
    /// was linked first, exports and all. Tagging the resolved name with this
    /// makes each registry a distinct module as far as that cache is concerned,
    /// the same trick a reload uses to re-read an application's own files.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Registers a module, building its functions through `build`.
    ///
    /// Registering the same name twice replaces the earlier module rather than
    /// merging into it: two registrations of one name are a mistake, and
    /// merging would hide it behind a module that half works.
    ///
    /// A name in [`RESERVED_SPECIFIERS`] is refused and reported by
    /// [`Self::validate`], which [`crate::export_modules`] calls. Refusing here
    /// and reporting there is what keeps this a builder: the host writes the
    /// list it means, and hears about every bad name at once rather than
    /// unwrapping each `register` call.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        build: impl FnOnce(&mut HostModule),
    ) -> &mut Self {
        let name = name.into();
        if RESERVED_SPECIFIERS.contains(&name.as_str()) {
            self.rejected.push(name);
            return self;
        }
        let mut module = HostModule::new(name.clone());
        build(&mut module);
        self.modules.insert(name, module);
        self
    }

    /// Reports every name [`Self::register`] refused, and every module whose
    /// TypeScript face disagrees with what it registered.
    ///
    /// The second check is the reason [`HostModule::declarations`] lives in
    /// Rust. Two files describing one boundary drift, and the drift shows up as
    /// an editor that completes a function the host deleted. Here it is a
    /// sentence at start-up.
    pub fn validate(&self) -> Result<(), HostError> {
        if !self.rejected.is_empty() {
            let names: Vec<&str> = self.rejected.iter().map(String::as_str).collect();
            return Err(HostError::new(format!(
                "these module names belong to the runtime and cannot be registered: {}. \
                 The reserved names are: {}",
                list(&names),
                list(RESERVED_SPECIFIERS)
            )));
        }

        for module in self.modules.values() {
            module.check_declarations()?;
        }
        Ok(())
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
    pub fn get(&self, name: &str) -> Result<&HostModule, HostError> {
        if let Some(module) = self.modules.get(name) {
            return Ok(module);
        }

        Err(HostError::new(if self.modules.is_empty() {
            format!(
                "host module `{name}` is not available: this host registered none. \
                 Host modules are granted by the embedding application, with \
                 gpui_shell::export_modules(...)."
            )
        } else {
            format!(
                "unknown host module `{name}`; this host registered: {}",
                list(&self.module_names())
            )
        }))
    }

    /// Resolves and calls in one step, for a host driving the registry
    /// directly. The engine goes through its guarded dispatcher instead.
    pub fn call(&self, module: &str, function: &str, arguments: &HostArguments) -> HostResult {
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
/// Must be called before the application is loaded, because a script's imports
/// are resolved against this registry while its module graph is linked. What
/// happens *after* that is unchanged from the lookup this replaced: an export
/// forwards through [`dispatch`] on every call, so revoking a module takes
/// effect on the next call rather than on the next restart.
pub(crate) fn set_modules(modules: HostModules) -> Result<(), HostError> {
    modules.validate()?;
    crate::policy::update_default(|policy| policy.with_host_modules(modules));
    Ok(())
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
    // An empty registry rejects nothing, so this cannot fail.
    let _ = set_modules(HostModules::default());
}

/// The registry the code now running may reach.
///
/// Read through the calling frame, so a plugin sees the modules its own host
/// registered for it rather than whichever set was installed most recently.
pub(crate) fn modules() -> Rc<HostModules> {
    crate::scope::policy().modules()
}

/// The one path from an engine into host code.
///
/// Refuses a nested call: a host function that has found a way to run script
/// code — and so to reach a second host function — has re-entered the engine,
/// which the module header explains is not allowed. Reporting it is the whole
/// value here; the alternative is a re-entrant render pass that fails somewhere
/// else entirely.
pub(crate) fn dispatch(module: &str, function: &str, arguments: &HostArguments) -> HostResult {
    if IN_CALL.with(Cell::get) {
        return Err(HostError::new(format!(
            "`{module}.{function}` was reached from inside another host call: \
             a host function may not call back into the script engine"
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

    fn registry() -> HostModules {
        let mut modules = HostModules::new();
        modules.register("workspace", |module| {
            module.function("project_name", |_| Ok(HostValue::from("gpui-component")));
            module.function("open_count", |arguments| {
                Ok(HostValue::from(arguments.len()))
            });
            module.function("echo", |arguments| Ok(arguments.value(0)?.clone()));
            module.function("close", |arguments| {
                let id = arguments.integer(0)?;
                Err(HostError::new(format!("tab {id} is already closed")))
            });
        });
        modules.register("editor", |module| {
            module.function("line_count", |_| Ok(HostValue::from(12)));
        });
        modules
    }

    #[test]
    fn a_registered_function_is_callable_and_returns_its_value() {
        let modules = registry();
        assert_eq!(
            modules
                .call("workspace", "project_name", &HostArguments::default())
                .unwrap(),
            HostValue::Str("gpui-component".into())
        );
        assert_eq!(
            modules
                .call(
                    "workspace",
                    "open_count",
                    &HostArguments::new([HostValue::from(1), HostValue::from(2)])
                )
                .unwrap(),
            HostValue::Number(2.)
        );
    }

    #[test]
    fn an_unregistered_module_reports_the_registered_ones() {
        let error = registry()
            .call("workspce", "project_name", &HostArguments::default())
            .unwrap_err();
        assert_eq!(
            error.message(),
            "unknown host module `workspce`; this host registered: editor, workspace"
        );
    }

    #[test]
    fn an_empty_registry_says_the_host_granted_nothing() {
        let error = HostModules::new()
            .call("workspace", "project_name", &HostArguments::default())
            .unwrap_err();
        assert!(error.message().contains("this host registered none"));
        assert!(error.message().contains("export_modules"));
    }

    #[test]
    fn an_unknown_function_reports_what_the_module_provides() {
        let error = registry()
            .call("editor", "line_cont", &HostArguments::default())
            .unwrap_err();
        assert_eq!(
            error.message(),
            "host module `editor` has no function `line_cont`; it provides: line_count"
        );
    }

    #[test]
    fn a_failing_function_surfaces_its_message() {
        let error = registry()
            .call(
                "workspace",
                "close",
                &HostArguments::new([HostValue::from(7)]),
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
                &HostArguments::new([HostValue::from("seven")]),
            )
            .unwrap_err();
        assert_eq!(error.message(), "argument 1 must be a number, got a string");
    }

    #[test]
    fn a_nested_value_round_trips_through_the_boundary_type() {
        let value: HostValue = HostObject::new()
            .field("name", "release")
            .field("done", true)
            .field("progress", 0.5)
            .field("owner", None::<String>)
            .field(
                "steps",
                vec![
                    HostValue::from(HostObject::new().field("id", 1).field("title", "Tag")),
                    HostValue::from(HostObject::new().field("id", 2).field("title", "Ship")),
                ],
            )
            .into();

        let returned = registry()
            .call("workspace", "echo", &HostArguments::new([value.clone()]))
            .unwrap();

        assert_eq!(returned, value);
        assert_eq!(
            returned.get("name").and_then(HostValue::as_str),
            Some("release")
        );
        assert!(returned.get("owner").is_some_and(HostValue::is_null));
        assert_eq!(
            returned
                .get("steps")
                .and_then(HostValue::as_array)
                .and_then(|steps| steps[1].get("title"))
                .and_then(HostValue::as_str),
            Some("Ship")
        );
    }

    #[test]
    fn a_repeated_field_replaces_the_earlier_value_in_place() {
        let value: HostValue = HostObject::new()
            .field("id", 1)
            .field("title", "Tag")
            .field("id", 2)
            .into();

        assert_eq!(
            value,
            HostValue::Object(vec![
                ("id".into(), HostValue::Number(2.)),
                ("title".into(), HostValue::Str("Tag".into())),
            ])
        );
    }

    #[test]
    fn a_reserved_name_is_refused_and_reported_by_name() {
        let mut modules = HostModules::new();
        modules.register("market", |module| {
            module.function("quotes", |_| Ok(HostValue::Null));
        });
        modules.register("path", |module| {
            module.function("join", |_| Ok(HostValue::Null));
        });
        modules.register("gpui", |module| {
            module.function("div", |_| Ok(HostValue::Null));
        });

        // The good one still registered; the reserved ones did not.
        assert_eq!(modules.module_names(), vec!["market"]);

        let error = modules.validate().unwrap_err();
        assert!(
            error.message().contains("path, gpui"),
            "{}",
            error.message()
        );
        assert!(
            error.message().contains("belong to the runtime"),
            "{}",
            error.message()
        );
    }

    /// Two registries are two modules to the engine's module cache, which is
    /// what keeps one plugin's `workspace` from being served to another.
    #[test]
    fn every_registry_has_its_own_generation() {
        let first = HostModules::new();
        let second = HostModules::new();
        assert_ne!(first.generation(), second.generation());
    }

    #[test]
    fn installing_a_reserved_name_fails_rather_than_silently_dropping_it() {
        let mut modules = HostModules::new();
        modules.register("console", |module| {
            module.function("log", |_| Ok(HostValue::Null));
        });
        assert!(set_modules(modules).is_err());
    }

    #[test]
    fn a_declared_face_that_matches_the_registry_passes() {
        let mut modules = HostModules::new();
        modules.register("market", |module| {
            module.function("quotes", |_| Ok(HostValue::Null));
            module.function("watch", |_| Ok(HostValue::Null));
            module.declarations(
                "export interface Quote { symbol: string }\n\
                 export function quotes(): Quote[];\n\
                 export function watch(symbol: string): boolean;\n",
            );
        });
        assert!(modules.validate().is_ok());
    }

    /// The drift this check exists for: the host renamed a function and the
    /// declarations still describe the old one.
    #[test]
    fn a_declared_face_that_drifts_names_both_sides() {
        let mut modules = HostModules::new();
        modules.register("market", |module| {
            module.function("quotes", |_| Ok(HostValue::Null));
            module.declarations("export function prices(): unknown[];\n");
        });

        let error = modules.validate().unwrap_err();
        assert!(
            error
                .message()
                .contains("registered but not declared: quotes"),
            "{}",
            error.message()
        );
        assert!(
            error
                .message()
                .contains("declared but not registered: prices"),
            "{}",
            error.message()
        );
    }

    /// A helper type is not an export, and must not be read as a missing one.
    #[test]
    fn only_exported_functions_are_read_as_exports() {
        let mut modules = HostModules::new();
        modules.register("market", |module| {
            module.function("quotes", |_| Ok(HostValue::Null));
            module.declarations(
                "// export function commented(): void;\n\
                 export interface Quote { symbol: string }\n\
                 type Row = Quote;\n\
                 export function quotes(): Quote[];\n",
            );
        });
        assert!(modules.validate().is_ok(), "{:?}", modules.validate());
    }

    #[test]
    fn registering_a_name_twice_replaces_the_module() {
        let mut modules = HostModules::new();
        modules.register("workspace", |module| {
            module.function("first", |_| Ok(HostValue::Null));
        });
        modules.register("workspace", |module| {
            module.function("second", |_| Ok(HostValue::Null));
        });

        assert_eq!(
            modules.get("workspace").unwrap().function_names(),
            vec!["second"]
        );
    }

    #[test]
    fn the_installed_registry_is_what_dispatch_calls() {
        set_modules(registry()).unwrap();

        assert_eq!(
            dispatch("editor", "line_count", &HostArguments::default()).unwrap(),
            HostValue::Number(12.)
        );

        set_modules(HostModules::new()).unwrap();
        assert!(modules().is_empty());
    }

    #[test]
    fn a_native_function_cannot_reach_a_second_one() {
        let mut modules = HostModules::new();
        modules.register("loop", |module| {
            module.function("outer", |_| {
                dispatch("loop", "inner", &HostArguments::default())
            });
            module.function("inner", |_| Ok(HostValue::Null));
        });
        set_modules(modules).unwrap();

        let error = dispatch("loop", "outer", &HostArguments::default()).unwrap_err();
        assert!(
            error
                .message()
                .contains("may not call back into the script engine"),
            "{}",
            error.message()
        );

        // The guard is released even after a refusal, so the next call works.
        assert_eq!(
            dispatch("loop", "inner", &HostArguments::default()).unwrap(),
            HostValue::Null
        );

        set_modules(HostModules::new()).unwrap();
    }
}
