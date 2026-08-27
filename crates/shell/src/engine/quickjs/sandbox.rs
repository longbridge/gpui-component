//! Sandbox policy: language trimming, capability-gated process access, and the
//! resource limits that keep a runaway application from taking the window with
//! it (design doc §19).
//!
//! # Ordering
//!
//! [`install`] must run **after** the runtime's own globals are installed — the
//! JS prelude, `host` and `scheduler` — and **before** any application module is
//! evaluated. Both halves matter. Running it earlier would let the prelude's own
//! writes land on prototypes that are supposed to be frozen by then, and would
//! let a later subsystem re-add a global this module means to withhold. Running
//! it later would mean application code had already had a turn with `eval` and a
//! mutable `Object.prototype`. `install_globals` in the parent module is the one
//! call site, and it is placed last for exactly this reason.
//!
//! # What this module does and does not own
//!
//! - It owns the *language* surface: dynamic code, the shared built-in
//!   prototypes, the stubs for globals that do not exist here.
//! - It owns `process`, the capability-gated command and exit surface.
//! - It does **not** own filesystem access. `fs/promises` uses [`super::host`]
//!   and goes through the same [`crate::capability::Capabilities`] resolver, so
//!   there is one path policy, not two.
//! - It does **not** re-check module resolution. `AppModules` in the parent
//!   module already confines both static `import` and dynamic `import()` to the
//!   application root; a second check here would be a second source of truth for
//!   the same rule. Dynamic `import()` is deliberately left callable — it is how
//!   §18.2 does lazy module loading.
//!
//! # Wiring the engine still needs
//!
//! The resource limits belong to the `Runtime`, which this module does not own.
//! The engine has to apply them itself, right after `JsRuntime::new()`:
//!
//! ```ignore
//! js_runtime.set_memory_limit(sandbox::memory_limit_bytes());
//! js_runtime.set_max_stack_size(sandbox::max_stack_size_bytes());
//! js_runtime.set_interrupt_handler(Some(Box::new(sandbox::interrupt_handler())));
//! ```
//!
//! One more, and it is the stronger half of the dynamic-code policy: QuickJS
//! makes evaluation an *optional intrinsic*, so a context assembled without it
//! has no `eval` and no `Function` compiler to reach in the first place —
//! `JsContext::custom::<(Date, RegExpCompiler, RegExp, Json, Proxy, MapSet,
//! TypedArrays, Promise, Performance, WeakRef)>(&js_runtime)` in place of
//! `JsContext::full`. That is `intrinsic::All` minus `intrinsic::Eval`.
//!
//! It is not done here because this module only receives a `Ctx`, and because
//! it is not free: `Ctx::eval` is `JS_Eval`, which the same intrinsic gates, so
//! dropping it also disables the engine's own `ctx.eval` — the JS prelude and
//! the two snippets below would have to become `Module::evaluate` calls or
//! precompiled bytecode first. Until that happens [`WITHHOLD_DYNAMIC_CODE`] is
//! the layer that is actually in force.

use std::{
    cell::Cell,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use rquickjs::{
    Ctx, Exception, Function, IntoJs, Object, Promise, Result as JsResult, Value,
    function::{Args, Func, Opt, Rest},
};

use crate::{
    capability::CapabilityError,
    scope::{self, ScopePhase},
};

use super::{host, scheduler};

/// Installs the sandbox policy into one context.
///
/// See the module header for the ordering requirement — this is the last thing
/// `install_globals` does.
pub fn install(ctx: &Ctx<'_>) -> JsResult<()> {
    install_absent_globals(ctx)?;

    if !is_development_mode() {
        ctx.eval::<(), _>(WITHHOLD_DYNAMIC_CODE)?;

        // After the constructor swap above, so the replacement is the thing that
        // gets frozen in place.
        if STRICT_BUILTINS.load(Ordering::Relaxed) {
            ctx.eval::<(), _>(FREEZE_BUILTINS)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Policy switches
// ---------------------------------------------------------------------------

static STRICT_BUILTINS: AtomicBool = AtomicBool::new(true);
static DEVELOPMENT_MODE: AtomicBool = AtomicBool::new(false);

// `mod sandbox` is private to the engine, so the host-facing entry points in
// this file — the two policy switches, the three resource limits and the exit
// request — have no caller yet. They become reachable when `engine` re-exports
// them for the host binary's `--dev` flag and runtime setup.

/// Whether the built-in prototypes are frozen. On by default.
///
/// Freezing is not free of cost to applications: a library that patches
/// `Array.prototype` — a polyfill, an older utility bundle — stops working, and
/// it stops working at import time with a `TypeError` that points at the
/// library rather than at this policy. That is the deliberate trade. One VM
/// hosts several plugins (§27), so the built-in prototypes are shared mutable
/// state; one plugin adding an enumerable property to `Object.prototype`
/// changes `for...in` for every other plugin and for the shell's own prelude.
/// A host that knowingly runs such a library can turn the freeze off, and keeps
/// every other part of the sandbox.
///
/// Read when a context is created, so call this before [`crate::ShellRuntime`]
/// is constructed.
#[allow(dead_code)]
pub fn set_strict_builtins(enabled: bool) {
    STRICT_BUILTINS.store(enabled, Ordering::Relaxed);
}

/// Turns on the relaxations a `--dev` host flag wants: `eval` and the `Function`
/// constructor come back, and the built-in prototypes are left writable so a
/// DevTools REPL can patch things while debugging.
///
/// Capability gating is *not* relaxed. Development mode makes the language
/// easier to poke at; it never hands out filesystem or process access that the
/// manifest did not declare, because a grant the author never wrote down is a
/// grant that will be missing in production.
///
/// §19.4 requires the host to keep a visible "development mode" marker in the
/// interface for as long as this is on.
#[allow(dead_code)]
pub fn set_development_mode(enabled: bool) {
    DEVELOPMENT_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_development_mode() -> bool {
    DEVELOPMENT_MODE.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Language trimming
// ---------------------------------------------------------------------------

/// Withholds every path from a string to executable code.
///
/// `globalThis.eval` is deleted outright: a `ReferenceError` cannot be mistaken
/// for a working `eval` by feature detection, which a throwing stub would be.
///
/// The `Function` constructor is *replaced* rather than deleted, because
/// `Function` is load-bearing for reasons unrelated to eval — `x instanceof
/// Function` and `Function.prototype.{call,apply,bind}` are ordinary, legitimate
/// JavaScript. The replacement keeps the real `Function.prototype` as its
/// `.prototype`, so those keep working and only construction throws.
///
/// Deleting `globalThis.Function` alone would achieve nothing anyway:
/// `(function () {}).constructor` is the same object, and each of the async,
/// generator and async-generator function prototypes carries its own
/// constructor which is an independent compiler. All four are swapped here.
///
/// This is the weaker of the two available layers, and it is deliberate that it
/// is the one implemented: the stronger fix is to assemble the context without
/// QuickJS's `Eval` intrinsic, which only the parent module can do. See the
/// module header for what that costs.
const WITHHOLD_DYNAMIC_CODE: &str = r#"
(() => {
  const hint = "the shell sandbox withholds dynamic code; run the host with --dev to allow it";
  const deny = (label) => function () {
    throw new TypeError(`${label} is disabled: ${hint}`);
  };

  const replaceConstructor = (holder, value) => {
    // `GeneratorFunction.prototype.constructor` is non-writable but
    // configurable, so plain assignment is not enough.
    Object.defineProperty(Object.getPrototypeOf(holder), "constructor", {
      value,
      writable: true,
      enumerable: false,
      configurable: true,
    });
  };

  const blocked = deny("the Function constructor");
  blocked.prototype = Function.prototype;

  replaceConstructor(function () {}, blocked);
  replaceConstructor(async function () {}, deny("the AsyncFunction constructor"));
  replaceConstructor(function* () {}, deny("the GeneratorFunction constructor"));
  replaceConstructor(async function* () {}, deny("the AsyncGeneratorFunction constructor"));

  globalThis.Function = blocked;
  delete globalThis.eval;
})();
"#;

/// Freezes the prototypes that every piece of script code shares.
///
/// Runs after [`WITHHOLD_DYNAMIC_CODE`] so the swapped `constructor` is what
/// gets frozen — freezing first would make the swap impossible.
const FREEZE_BUILTINS: &str = r#"
(() => {
  for (const proto of [
    Object.prototype,
    Array.prototype,
    Function.prototype,
    String.prototype,
    Number.prototype,
  ]) {
    Object.freeze(proto);
  }
})();
"#;

/// Names an author will reach for that this runtime does not have.
///
/// Having the name present and throwing a message that points at the
/// replacement beats a bare `ReferenceError`, which says only that the name is
/// missing and nothing about what to write instead (§2.1, §19.1).
///
/// The DOM names — `window`, `document`, `localStorage` — are deliberately *not*
/// stubbed. They are read through `typeof` by every bundle that does environment
/// detection, and `typeof window === "undefined"` is the answer that makes such
/// a bundle take its non-browser branch. A throwing getter would turn a working
/// feature test into a crash.
const ABSENT_GLOBALS: &[(&str, &str)] = &[
    ("setTimeout", "use cx.timer.after(ms, callback)"),
    ("setInterval", "use cx.timer.every(ms, callback)"),
    (
        "clearTimeout",
        "cancel the handle returned by cx.timer.after",
    ),
    (
        "clearInterval",
        "cancel the handle returned by cx.timer.every",
    ),
    ("require", "this runtime uses ES modules; use `import`"),
];

/// Installs the throwing stubs, skipping any name a subsystem has already
/// claimed. `scheduler` may one day provide a real `setTimeout`; if it does, it
/// wins, and this module does not have to be edited to notice.
fn install_absent_globals(ctx: &Ctx<'_>) -> JsResult<()> {
    let globals = ctx.globals();
    for (name, hint) in ABSENT_GLOBALS {
        if globals.contains_key(*name)? {
            continue;
        }

        let message = format!("`{name}` is not available in the shell: {hint}");
        globals.set(
            *name,
            Func::from(move |ctx: Ctx<'_>| -> JsResult<()> {
                Err(Exception::throw_type(&ctx, &message))
            }),
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Capability-gated process access
// ---------------------------------------------------------------------------

/// Installs `process.run` and `process.exit`.
///
/// The project's ruling is that process and filesystem operations stay
/// *reachable* and are gated, rather than being removed: an author who declares
/// the capability gets the operation under its ordinary name, and an author who
/// does not gets an error naming the manifest key to add. Removing them instead
/// would produce a `ReferenceError` that teaches nothing.
///
/// `process` is a global as well as a bare module, because `process` is
/// the name a JavaScript author — or a model generating JavaScript — will reach
/// for. The module loader and this installer share the same policy-aware
/// implementation.
///
/// Runs a command and resolves with what it did.
///
/// A promise, for a sharper version of the reason `fs/promises` is one. A file read
/// has no bound; a child process has *less* — it can compute for minutes, wait
/// on input that never comes, or outlive the window. `.status()` blocked the
/// frame and the VM together for as long as that took, in the kernel, where the
/// interrupt budget cannot see it. So the wait happens on the background
/// executor and the promise settles on the main thread.
///
/// The capability check stays here, on the calling thread, so a denial is still
/// a thrown error at the call site rather than a rejected promise nobody
/// awaited.
///
/// Output is captured rather than inherited. A script that runs a command
/// almost always wants what it said, and in a windowed application a child
/// writing to the host's stdout is writing to nowhere a user will look.
fn run<'js>(ctx: Ctx<'js>, command: String, args: Opt<Vec<String>>) -> JsResult<Promise<'js>> {
    if !host::capabilities().may_run(&command) {
        return Err(Exception::throw_type(
            &ctx,
            &CapabilityError::ExecuteDenied(command).to_string(),
        ));
    }

    let args = args.0.unwrap_or_default();
    let cancellation = crate::process::Cancellation::new();
    let worker_cancellation = cancellation.clone();
    scheduler::blocking_cancellable(
        &ctx,
        "process.run(command, args)",
        move || cancellation.cancel(),
        move || {
            crate::process::run_bounded(
                &command,
                &args,
                crate::process::Limits::default(),
                worker_cancellation,
            )
        },
    )
}

fn next_tick<'js>(ctx: Ctx<'js>, callback: Function<'js>, args: Rest<Value<'js>>) -> JsResult<()> {
    let mut deferred = Args::new(ctx, args.len());
    for argument in args.0 {
        deferred.push_arg(argument)?;
    }
    callback.defer_arg(deferred)
}

/// What a command did: its status, and both streams.
///
/// A plain struct because it crosses threads — the work runs on the background
/// executor and only the result comes back — so the conversion to a script value
/// happens here, on the main thread, the way `DirEntry` does in `host`.
impl<'js> IntoJs<'js> for crate::process::Output {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        let object = Object::new(ctx.clone())?;
        object.set("code", self.code)?;
        object.set("stdout", self.stdout)?;
        object.set("stderr", self.stderr)?;
        Ok(object.into_value())
    }
}

pub(super) fn install_process(ctx: &Ctx<'_>) -> JsResult<()> {
    let process = Object::new(ctx.clone())?;

    process.set("run", Func::from(run))?;
    process.set("nextTick", Func::from(next_tick))?;

    process.set(
        "exit",
        Func::from(|ctx: Ctx<'_>, code: Opt<i32>| -> JsResult<()> {
            let capabilities = host::capabilities();
            if !capabilities.may_exit() {
                return Err(Exception::throw_type(
                    &ctx,
                    "process.exit() is not granted; set capabilities.process.exit to true in the manifest",
                ));
            }

            // A request, never `exit(2)`: one plugin must not be able to take
            // the host process down, and the host may have unsaved state. What
            // the request *does* is the host's to decide — and a host that
            // granted the capability without deciding is told here, rather than
            // handing the script a success while nothing happens.
            let Some(handler) = crate::runtime::exit_handler() else {
                return Err(Exception::throw_message(
                    &ctx,
                    "process.exit() is granted but this host installed no handler for it; \
                     a host that grants exit has to say what exit means \
                     (gpui_shell::on_exit_request)",
                ));
            };

            let request =
                crate::runtime::ExitRequest::new(code.0.unwrap_or(0), crate::scope::current_view());
            crate::scope::with_current(|window, cx| handler(request, window, cx)).ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    "process.exit() needs a live host call; call it from init(), an event \
                     handler or a task",
                )
            })
        }),
    )?;

    process.set("platform", std::env::consts::OS)?;
    process.set("arch", std::env::consts::ARCH)?;
    ctx.globals().set("process", process)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Resource limits
// ---------------------------------------------------------------------------

/// The heap ceiling for one QuickJS runtime.
///
/// Generous enough that an ordinary application never meets it, small enough
/// that a leak reports as a catchable JavaScript exception on the offending
/// allocation instead of as an OOM kill of the whole host (§19.3).
///
/// The engine owns the `Runtime`, so it has to apply this — see the module
/// header for the exact lines.
#[allow(dead_code)]
pub fn memory_limit_bytes() -> usize {
    256 * 1024 * 1024
}

/// The interpreter stack ceiling.
///
/// QuickJS's default is 256 KiB. Deep recursion in an application should surface
/// as a `RangeError` the script can report, not as a native stack overflow,
/// which is a process abort and takes the host with it.
#[allow(dead_code)]
pub fn max_stack_size_bytes() -> usize {
    1024 * 1024
}

/// How long one host call may keep the interpreter before it is cut off.
///
/// Kept private: the numbers are policy, not API, and tests need a version with
/// a millisecond deadline so they finish quickly.
#[derive(Clone, Copy)]
struct Budgets {
    /// Render and layout run inside GPUI's frame; overrunning here is visible
    /// as a dropped frame, so the budget is the tight one.
    render: Duration,
    /// An event handler may legitimately do real work before it returns.
    event: Duration,
    /// Module evaluation at startup, which happens outside any call scope.
    detached: Duration,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            render: Duration::from_millis(50),
            event: Duration::from_millis(500),
            detached: Duration::from_secs(5),
        }
    }
}

/// A deadline-based interrupt handler for `Runtime::set_interrupt_handler`.
///
/// QuickJS calls this periodically while interpreting; returning `true` unwinds
/// the current execution. The budget is per host call rather than global:
/// [`scope::enter`] identifies scoped render/event/task calls, while
/// [`begin_host_execution`] identifies prelude and module evaluations outside
/// a scope. A change of either identity restarts the clock. That is what lets
/// the render path have a tighter budget than an event handler without the
/// handler needing to be reinstalled between calls.
///
/// The engine owns the `Runtime`, so it has to install this — see the module
/// header.
#[allow(dead_code)]
pub fn interrupt_handler() -> impl FnMut() -> bool + 'static {
    deadline(Budgets::default())
}

thread_local! {
    /// Monotonic identity for host-initiated evaluations that have no GPUI call
    /// scope. Scoped calls already carry their own generation in `scope`.
    static DETACHED_EXECUTION: Cell<u64> = const { Cell::new(0) };
}

/// Starts one host-initiated entry into the JavaScript context.
///
/// Module evaluation and the runtime prelude execute without a GPUI call
/// scope, so [`scope::current_generation`] cannot distinguish one from the
/// next. Advancing this epoch gives each such evaluation its own interrupt
/// budget. Render, layout, event, and task calls continue to use their scope
/// generation, so nested engine helpers cannot refresh those tighter budgets.
pub(super) fn begin_host_execution() {
    DETACHED_EXECUTION.with(|epoch| {
        epoch.set(
            epoch
                .get()
                .checked_add(1)
                .expect("gpui-shell exhausted detached execution epochs"),
        );
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExecutionWindow {
    Scoped(u64),
    Detached(u64),
}

fn deadline(budgets: Budgets) -> impl FnMut() -> bool + 'static {
    let mut window = None;
    let mut started = Instant::now();

    move || {
        let current = match scope::current_generation() {
            Some(generation) => ExecutionWindow::Scoped(generation),
            None => DETACHED_EXECUTION.with(|epoch| ExecutionWindow::Detached(epoch.get())),
        };
        if window != Some(current) {
            window = Some(current);
            started = Instant::now();
        }

        let budget = match scope::current_phase() {
            Some(ScopePhase::Render | ScopePhase::Layout) => budgets.render,
            Some(ScopePhase::Event | ScopePhase::Task) => budgets.event,
            None => budgets.detached,
        };

        started.elapsed() > budget
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use rquickjs::{Context as JsContext, Runtime as JsRuntime, Value};

    use super::*;

    /// The policy switches are process-wide, so tests that flip them have to be
    /// serialized. The guard also restores the defaults, so a test never
    /// inherits the previous one's mode.
    fn policy() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        set_development_mode(false);
        set_strict_builtins(true);
        guard
    }

    /// A context with the sandbox installed, matching the ordering the engine
    /// uses: full intrinsics first, policy last.
    fn sandboxed() -> (JsRuntime, JsContext) {
        let runtime = JsRuntime::new().unwrap();
        let context = JsContext::full(&runtime).unwrap();
        context.with(|ctx| {
            install_process(&ctx).unwrap();
            install(&ctx).unwrap();
        });
        (runtime, context)
    }

    /// The message of the exception a piece of script threw, or `None` if it
    /// did not throw.
    fn rejection(context: &JsContext, source: &str) -> Option<String> {
        context.with(|ctx| {
            let result = ctx.eval::<Value, _>(source);
            result.err()?;
            let caught = ctx.catch();
            Some(
                caught
                    .as_exception()
                    .and_then(|exception| exception.message())
                    .unwrap_or_else(|| format!("{caught:?}")),
            )
        })
    }

    #[test]
    fn eval_is_withheld_and_returns_under_development_mode() {
        let _policy = policy();

        let (_runtime, context) = sandboxed();
        assert!(rejection(&context, "eval('1 + 1')").is_some());
        // The host's own evaluation path is `JS_Eval`, not the global, so
        // removing the global does not disarm the engine itself.
        let still_works: i32 = context.with(|ctx| ctx.eval("1 + 1").unwrap());
        assert_eq!(still_works, 2);

        set_development_mode(true);
        let (_runtime, dev) = sandboxed();
        let evaluated: i32 = dev.with(|ctx| ctx.eval("eval('1 + 1')").unwrap());
        assert_eq!(evaluated, 2);
    }

    #[test]
    fn every_function_constructor_is_closed_as_an_eval_path() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();

        for source in [
            "new Function('return 1')()",
            "Function('return 1')()",
            "(function () {}).constructor('return 1')()",
            "(() => {}).constructor('return 1')()",
            "(async function () {}).constructor('return 1')",
            "(function* () {}).constructor('return 1')",
            "(async function* () {}).constructor('return 1')",
        ] {
            assert!(
                rejection(&context, source).is_some(),
                "`{source}` is a way back to a compiler"
            );
        }
    }

    #[test]
    fn the_function_stub_keeps_the_ordinary_uses_of_function_working() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();

        let intact: bool = context
            .with(|ctx| {
                ctx.eval(
                    "((f) => f instanceof Function && f.call(null, 1) === 1 && \
                      typeof f.bind(null) === 'function')((x) => x)",
                )
            })
            .unwrap();
        assert!(intact);
    }

    #[test]
    fn strict_builtins_defeat_prototype_pollution() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();

        // Application code is module code, which is strict by definition, so a
        // write to a frozen prototype is a `TypeError` the author sees.
        assert!(rejection(&context, "Object.prototype.polluted = 1;").is_some());
        assert!(rejection(&context, "Array.prototype.first = () => 1;").is_some());

        // A sloppy-mode write discards silently instead of throwing — that is
        // ECMAScript's rule, not a hole in the policy. Assert the outcome that
        // actually matters: the property never appears.
        let leaked: bool = context
            .with(|ctx| {
                let mut options = rquickjs::context::EvalOptions::default();
                options.strict = false;
                ctx.eval_with_options(
                    "Object.prototype.polluted = 1; ({}).polluted !== undefined",
                    options,
                )
            })
            .unwrap();
        assert!(!leaked);
    }

    #[test]
    fn the_freeze_is_switchable_for_hosts_that_need_it() {
        let _policy = policy();
        set_strict_builtins(false);
        let (_runtime, context) = sandboxed();

        let patched: bool = context
            .with(|ctx| {
                ctx.eval(
                    "Array.prototype.first = function () { return this[0]; }; \
                                  [7].first() === 7",
                )
            })
            .unwrap();
        assert!(patched);

        // Turning the freeze off must not hand back a compiler.
        assert!(rejection(&context, "new Function('return 1')").is_some());
    }

    #[test]
    fn process_run_refuses_without_a_grant_and_names_the_manifest_key() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();

        let message = rejection(&context, "process.run('git', ['status'])")
            .expect("process.run must refuse an undeclared command");
        assert!(
            message.contains("capabilities.fs.execute"),
            "the refusal must name the key to declare, got: {message}"
        );
        assert!(message.contains("git"), "and the command, got: {message}");
    }

    #[test]
    fn filesystem_access_does_not_grant_process_exit() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();
        crate::capability::install(
            crate::capability::Capabilities::new().read_roots([std::env::temp_dir()]),
        );
        crate::runtime::clear_exit_handler();

        let message = rejection(&context, "process.exit(0)")
            .expect("filesystem access must not grant process.exit");
        assert!(
            message.contains("capabilities.process.exit"),
            "the refusal must name the key to declare, got: {message}"
        );
    }

    /// A granted exit that nobody answers used to be a success that did
    /// nothing: the code went into a cell no production caller ever read. The
    /// script cannot tell the difference between that and a working exit, which
    /// is the worst shape a no-op can take.
    #[test]
    fn a_granted_exit_without_a_handler_says_so_rather_than_succeeding() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();
        crate::capability::install(crate::capability::Capabilities::new().exit(true));
        crate::runtime::clear_exit_handler();

        let message = rejection(&context, "process.exit(3)")
            .expect("a granted exit with no handler must not report success");
        assert!(
            message.contains("no handler"),
            "the failure must name the host's omission, got: {message}"
        );
    }

    #[test]
    fn withheld_globals_report_what_to_use_instead() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();

        let message = rejection(&context, "setTimeout(() => {}, 0)").unwrap();
        assert!(message.contains("cx.timer.after"), "got: {message}");
        let message = rejection(&context, "setInterval(() => {}, 0)").unwrap();
        assert!(message.contains("cx.timer.every"), "got: {message}");
        let message = rejection(&context, "clearTimeout(1)").unwrap();
        assert!(message.contains("cx.timer.after"), "got: {message}");
        let message = rejection(&context, "clearInterval(1)").unwrap();
        assert!(message.contains("cx.timer.every"), "got: {message}");

        // Environment detection has to keep working, so the DOM names stay
        // absent rather than becoming throwing stubs.
        let detected: bool = context
            .with(|ctx| {
                ctx.eval("typeof window === 'undefined' && typeof document === 'undefined'")
            })
            .unwrap();
        assert!(detected);
    }

    #[test]
    fn quickjs_libc_is_not_reachable() {
        let _policy = policy();
        let (_runtime, context) = sandboxed();

        // `std` and `os` are quickjs-libc, and rquickjs-sys does not compile
        // that file at all — this is a regression guard on the build, not a
        // check on anything this module removed.
        let absent: bool = context
            .with(|ctx| ctx.eval("typeof std === 'undefined' && typeof os === 'undefined'"))
            .unwrap();
        assert!(absent);

        // Dynamic `import()` stays callable on purpose (§18.2 lazy loading) and
        // reports asynchronously, so a bad specifier is a rejected promise
        // rather than a thrown exception. Confining it is the resolver's job,
        // not this module's.
        let deferred: bool = context
            .with(|ctx| ctx.eval("import('std') instanceof Promise"))
            .unwrap();
        assert!(deferred);
    }

    fn interrupting_runtime(budget: Duration) -> JsRuntime {
        let runtime = JsRuntime::new().unwrap();
        runtime.set_interrupt_handler(Some(Box::new(deadline(Budgets {
            render: budget,
            event: budget,
            detached: budget,
        }))));
        runtime
    }

    #[test]
    fn the_interrupt_handler_stops_an_infinite_loop() {
        let runtime = interrupting_runtime(Duration::from_millis(20));
        let context = JsContext::full(&runtime).unwrap();

        let started = Instant::now();
        let interrupted = context.with(|ctx| ctx.eval::<Value, _>("while (true) {}").is_err());

        assert!(interrupted, "a bare infinite loop must be cut off");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "and it must be cut off promptly"
        );
    }

    #[test]
    fn detached_host_executions_receive_independent_budgets() {
        let budget = Duration::from_millis(200);
        let runtime = interrupting_runtime(budget);
        let context = JsContext::full(&runtime).unwrap();

        for _ in 0..2 {
            begin_host_execution();
            context
                .with(|ctx| {
                    ctx.eval::<(), _>(
                        "{ const until = Date.now() + 120; while (Date.now() < until) {} }",
                    )
                })
                .expect("each detached evaluation should receive a fresh budget");
        }
    }

    /// §19.3 requires this to be measured rather than assumed: if a script could
    /// swallow the interrupt, the interrupt would not be a defence at all and
    /// the policy would have to escalate to discarding the whole context.
    #[test]
    fn an_interrupt_cannot_be_swallowed_by_a_catch_block() {
        let runtime = interrupting_runtime(Duration::from_millis(20));
        let context = JsContext::full(&runtime).unwrap();

        let started = Instant::now();
        let escaped = context.with(|ctx| {
            ctx.eval::<Value, _>("try { while (true) {} } catch (error) { 'swallowed' }")
                .is_ok()
        });

        assert!(!escaped, "an interrupt must not be catchable from script");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn the_render_budget_is_tighter_than_the_event_budget() {
        let budgets = Budgets::default();
        assert!(budgets.render < budgets.event);
        assert!(budgets.event < budgets.detached);
    }
}
