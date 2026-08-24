//! Promises, timers and task ownership on GPUI's foreground executor
//! (design doc §12).
//!
//! Script code is asynchronous in the ordinary JavaScript way: `async` functions
//! and native promises. What this module supplies is the half a bare QuickJS
//! runtime does not have — a clock, an owner for pending work, and somebody to
//! pump the job queue.
//!
//! # A `cx` may not be held across an `await`
//!
//! This is the one rule an application author has to internalize:
//!
//! ```js
//! async run(cx) {
//!   const data = await load();
//!   cx.notify();              // WRONG: this `cx` belongs to a call that returned
//!   gpui.with_cx((cx) => cx.notify());  // right
//! }
//! ```
//!
//! GPUI hands the host `&mut Window` and `&mut App` as borrows that live exactly
//! as long as one call. The script-side `cx` is only a generation token for that
//! call ([`crate::scope`]). An `await` returns to the host, the call frame goes
//! away, and the borrows with it — so the token is stale by the time the
//! continuation runs. Every resumption this module drives therefore opens a
//! *fresh* [`ScopePhase::Task`] scope, and [`js_with_cx`] is how resumed code
//! obtains a `cx` that belongs to it. Using the old one is not undefined
//! behaviour; it is a clear [`crate::scope::StaleContext`] error, which is the
//! whole point of the generation.
//!
//! # The job queue
//!
//! QuickJS keeps promise reactions in a job queue that only runs when the host
//! asks it to. Nothing — no `.then`, no continuation after an `await` — happens
//! until somebody calls [`drain_jobs`]. **The engine must call it after every
//! script entry point** (render, click, change, timer, load); see that
//! function's documentation for the exact placement, which is inside the entry
//! point's scope and outside `Context::with`.
//!
//! # Ownership and cancellation
//!
//! Every task belongs to a view: `opts.owner`, or [`scope::current_view`] at
//! creation. The task holds a [`WeakEntity`], so when the panel that started the
//! work is closed the callback is skipped instead of writing into state that
//! nothing will ever render again. Explicit `cancel()` does the same thing on
//! demand. For a promise this means the promise is simply never settled: a
//! cancelled task stops, it does not report an error nobody asked for.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use gpui::{AnyWindowHandle, AsyncApp, BackgroundExecutor, Entity, ForegroundExecutor, WeakEntity};
use rquickjs::{
    Ctx, Exception, FromJs, Function, Object, Persistent, Promise, Result as JsResult,
    Runtime as JsRuntime, Value,
    function::{Func, Opt, This},
};

use crate::{
    scope::{self, ScopePhase},
    view::ScriptView,
};

use super::{ShellRuntime, context_object, describe};

/// Installs `sleep`, `timer`, `spawn` and `with_cx` onto the `gpui` module.
///
/// The context argument is the engine's; everything here is built from the
/// module's own [`Object::ctx`], because `Ctx` and `Object` are invariant in
/// `'js` and a value built from one cannot be set on the other.
pub fn install(_ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    let ctx = module.ctx();
    module.set("sleep", Func::from(js_sleep))?;
    module.set("with_cx", Func::from(js_with_cx))?;
    module.set("spawn", Func::from(js_spawn))?;
    module.set("timer", timer_object(ctx)?)?;
    Ok(())
}

/// Runs every promise reaction QuickJS has queued.
///
/// # The engine must call this after every entry point
///
/// A pending job is script code that has already started: the tail of an
/// `async` function after its `await`, or a `.then` callback. If nobody drains
/// the queue, `await` never resumes and the failure is silent — no error, no
/// log, just a promise that stays pending. So every place that calls into
/// JavaScript ends with a drain:
///
/// - `render_view` / `render_to_spec`, after the render call;
/// - `dispatch_click` / `dispatch_change`, after the handler;
/// - `load_source` / `instantiate`, after top-level module evaluation;
/// - every resumption this module drives, which is handled here already.
///
/// Two placement rules, both load-bearing:
///
/// 1. **Outside `Context::with`.** [`JsRuntime::execute_pending_job`] takes the
///    runtime lock that `Context::with` is already holding, so calling it from
///    inside a `with_js` closure deadlocks (or, without the `parallel` feature,
///    panics on a re-entrant `RefCell` borrow).
/// 2. **Inside the entry point's [`crate::scope`] guard.** A continuation is
///    resumed script code and will call `gpui.with_cx`, which needs a live
///    scope. Draining after the guard drops leaves that code with no context at
///    all. For an entry point whose own phase is not `Task` — a render pass —
///    open a fresh `ScopePhase::Task` scope around the drain instead.
///
/// A job that throws is reported and the drain continues: one broken
/// continuation must not stop the others.
pub fn drain_jobs(runtime: &JsRuntime) {
    for _ in 0..MAX_JOBS_PER_DRAIN {
        match runtime.execute_pending_job() {
            Ok(true) => {}
            Ok(false) => return,
            // `JobException` is not a nameable type outside rquickjs, but its
            // context — the one that threw — is a public field.
            Err(exception) => {
                let message = exception.0.with(|ctx| {
                    let thrown = ctx.catch();
                    describe_value(&ctx, &thrown)
                });
                tracing::error!("error in a promise continuation: {message}");
            }
        }
    }

    tracing::error!(
        "stopped draining promise jobs after {MAX_JOBS_PER_DRAIN}: a continuation is queueing \
         work faster than it completes. The remaining jobs run at the next entry point."
    );
}

/// Releases every pending task and the script functions they hold.
///
/// The engine should call this from `ShellRuntime::drop`, next to
/// `callbacks.clear()` and for the same reason: a `Persistent` released after
/// its runtime aborts the process.
// Unused until `ShellRuntime::drop` calls it; see the module header's wiring
// notes, which is the one change outside this file the scheduler needs.
#[allow(dead_code)]
pub fn shutdown() {
    TASKS.with_borrow_mut(|tasks| {
        for task in tasks.values() {
            task.cancelled.set(true);
            task.callback.replace(None);
        }
        tasks.clear();
    });
}

/// A drain that is bounded so a `for(;;) Promise.resolve().then(f)` cannot wedge
/// the frame loop for ever. It is far above any legitimate burst.
const MAX_JOBS_PER_DRAIN: usize = 100_000;

// ---------------------------------------------------------------------------
// The JavaScript surface
// ---------------------------------------------------------------------------

/// `gpui.sleep(ms)` — a promise resolved after `ms` on the foreground executor.
///
/// The delay itself is a background timer, but nothing script-visible ever
/// leaves the main thread: the resolution runs in a fresh `Task` scope, and the
/// continuation it unblocks runs in the drain that follows, inside that same
/// scope.
///
/// A free function rather than a closure because the returned `Promise<'js>`
/// borrows the context lifetime; a closure cannot be inferred as polymorphic
/// over a lifetime that appears in both its parameter and its return type.
fn js_sleep<'js>(ctx: Ctx<'js>, ms: Opt<f64>) -> JsResult<Promise<'js>> {
    const API: &str = "gpui.sleep(ms)";

    let delay = duration(&ctx, API, ms.0.unwrap_or_default())?;
    let host = host(&ctx, API)?;
    let (promise, resolve, _reject) = ctx.promise()?;

    let task = register(TaskState::new(
        API,
        scope::current_view().map(|view| view.downgrade()),
        Some(Persistent::save(&ctx, resolve)),
    ));

    let sleeping = task.clone();
    host.foreground
        .clone()
        .spawn(async move {
            host.background.timer(delay).await;

            // A cancelled sleep leaves its promise pending for ever, which is
            // what cancellation means for a promise: the continuation does not
            // run, and no error is invented for code that asked to stop.
            if let Readiness::Ready(owner) = sleeping.readiness()
                && let Some(resolve) = sleeping.take_callback()
            {
                resume(&host, owner, move |ctx, _| {
                    resolve.restore(ctx)?.call::<_, ()>(())
                });
            }
            finish(&sleeping);
        })
        .detach();

    Ok(promise)
}

/// `gpui.with_cx(fn)` — runs `fn(cx)` with a context that belongs to the call in
/// progress.
///
/// This is how code that has `await`ed gets a usable `cx`: the one it was handed
/// before the `await` belongs to a call that has already returned. Outside any
/// host call there is no context to hand out, and saying so is far better than
/// producing one that points at a dead stack frame.
fn js_with_cx<'js>(ctx: Ctx<'js>, body: Function<'js>) -> JsResult<Value<'js>> {
    let Some(generation) = scope::current_generation() else {
        return Err(outside_host_call(&ctx, "gpui.with_cx(fn)"));
    };
    body.call((context_object(&ctx, generation)?,))
}

/// `gpui.spawn(asyncFn, opts?)` — calls `asyncFn(cx)` and adopts its promise.
///
/// Adoption is the point. A rejected promise nobody handles is JavaScript's most
/// common silent failure: the work stops, the interface keeps the state it had,
/// and nothing is written anywhere. Here it reaches `tracing::error!` with the
/// script's own stack.
///
/// The `cx` passed to `asyncFn` is valid until its first `await`, and only when
/// there is a host call in progress — at module top level there is no window
/// yet, and the argument is `undefined`.
fn js_spawn<'js>(
    ctx: Ctx<'js>,
    body: Function<'js>,
    opts: Opt<Value<'js>>,
) -> JsResult<Object<'js>> {
    let task = register(TaskState::new(
        "gpui.spawn(fn)",
        owner_from_options(&ctx, opts.0.as_ref())?,
        None,
    ));

    let started = match scope::current_generation() {
        Some(generation) => body.call::<_, Value>((context_object(&ctx, generation)?,)),
        None => body.call::<_, Value>(()),
    };

    match started {
        Ok(value) => match value.into_promise() {
            Some(promise) => adopt(&promise, &task)?,
            // A plain function, already finished. Nothing to wait for.
            None => finish(&task),
        },
        Err(error) => {
            tracing::error!("error in gpui.spawn: {}", describe(&ctx, error));
            finish(&task);
        }
    }

    handle_object(&ctx, task.id)
}

/// `gpui.timer.after(ms, fn, opts?)`.
fn js_timer_after<'js>(
    ctx: Ctx<'js>,
    ms: f64,
    handler: Function<'js>,
    opts: Opt<Value<'js>>,
) -> JsResult<Object<'js>> {
    schedule(
        ctx,
        "gpui.timer.after(ms, fn)",
        ms,
        handler,
        opts,
        Repeat::Once,
    )
}

/// `gpui.timer.every(ms, fn, opts?)`.
///
/// The interval is measured between the end of one callback and the start of the
/// next wait, so a slow callback delays the following tick rather than piling
/// ticks up behind it. §12.5 leaves throttling an invisible window to host
/// policy; this implementation keeps ticking.
fn js_timer_every<'js>(
    ctx: Ctx<'js>,
    ms: f64,
    handler: Function<'js>,
    opts: Opt<Value<'js>>,
) -> JsResult<Object<'js>> {
    schedule(
        ctx,
        "gpui.timer.every(ms, fn)",
        ms,
        handler,
        opts,
        Repeat::Forever,
    )
}

fn timer_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
    let timer = Object::new(ctx.clone())?;
    timer.set("after", Func::from(js_timer_after))?;
    timer.set("every", Func::from(js_timer_every))?;
    Ok(timer)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Once,
    Forever,
}

fn schedule<'js>(
    ctx: Ctx<'js>,
    api: &'static str,
    ms: f64,
    handler: Function<'js>,
    opts: Opt<Value<'js>>,
    repeat: Repeat,
) -> JsResult<Object<'js>> {
    let delay = duration(&ctx, api, ms)?;
    let host = host(&ctx, api)?;
    let task = register(TaskState::new(
        api,
        owner_from_options(&ctx, opts.0.as_ref())?,
        Some(Persistent::save(&ctx, handler)),
    ));

    let ticking = task.clone();
    host.foreground
        .clone()
        .spawn(async move {
            loop {
                host.background.timer(delay).await;

                let Readiness::Ready(owner) = ticking.readiness() else {
                    break;
                };
                let callback = match repeat {
                    Repeat::Once => ticking.take_callback(),
                    Repeat::Forever => ticking.clone_callback(),
                };
                let Some(callback) = callback else { break };

                resume(&host, owner, move |ctx, generation| {
                    callback
                        .restore(ctx)?
                        .call::<_, ()>((context_object(ctx, generation)?,))
                });

                if repeat == Repeat::Once {
                    break;
                }
            }
            finish(&ticking);
        })
        .detach();

    handle_object(&ctx, task.id)
}

/// The script-side task handle: `cancel()` and `is_done()`.
///
/// It carries only the id, so it stays valid after the task has been reaped.
fn handle_object<'js>(ctx: &Ctx<'js>, id: TaskId) -> JsResult<Object<'js>> {
    let handle = Object::new(ctx.clone())?;
    handle.set("cancel", Func::from(move || cancel(id)))?;
    handle.set("is_done", Func::from(move || is_done(id)))?;
    Ok(handle)
}

/// Attaches the reporting handlers to a spawned task's promise.
fn adopt<'js>(promise: &Promise<'js>, task: &Rc<TaskState>) -> JsResult<()> {
    let settled = task.clone();
    let failed = task.clone();

    // `then` rather than `catch` so success also marks the task done; the
    // success handler takes no arguments because the resolved value is not ours.
    promise.then()?.call::<_, ()>((
        This(promise.clone()),
        Func::from(move || finish(&settled)),
        Func::from(move |failure: ScriptFailure| {
            if !failed.cancelled.get() {
                tracing::error!("unhandled rejection in gpui.spawn: {}", failure.0);
            }
            finish(&failed);
        }),
    ))
}

/// A rejection value, already rendered as a message.
///
/// The formatting has to happen in `FromJs`: a closure taking both `Ctx<'js>`
/// and `Value<'js>` cannot be inferred, because the two elided lifetimes are
/// separate parameters as far as inference is concerned. Inside `from_js` they
/// are one lifetime again. `Arguments` in the parent module exists for the same
/// reason.
struct ScriptFailure(String);

impl<'js> FromJs<'js> for ScriptFailure {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        Ok(Self(describe_value(ctx, &value)))
    }
}

// ---------------------------------------------------------------------------
// Resumption
// ---------------------------------------------------------------------------

/// Everything a deferred task needs to get back onto the main thread.
///
/// Captured while a host call is in progress, because that is the only time the
/// window handle and the executors are reachable.
struct Host {
    window: AnyWindowHandle,
    app: AsyncApp,
    foreground: ForegroundExecutor,
    background: BackgroundExecutor,
}

fn host(ctx: &Ctx<'_>, api: &str) -> JsResult<Host> {
    let Some(generation) = scope::current_generation() else {
        return Err(outside_host_call(ctx, api));
    };

    scope::with_context(generation, |window, app| Host {
        window: window.window_handle(),
        app: app.to_async(),
        foreground: app.foreground_executor().clone(),
        background: app.background_executor().clone(),
    })
    .map_err(|error| Exception::throw_type(ctx, &error.to_string()))
}

/// Re-enters script code in a fresh [`ScopePhase::Task`] scope.
///
/// The three steps are one unit and their order is the contract: open the scope,
/// call the script, drain the jobs the call queued — all before the guard drops,
/// because a continuation resumed by that drain is script code that will ask for
/// a `cx` of its own.
fn resume(
    host: &Host,
    owner: Option<Entity<ScriptView>>,
    body: impl FnOnce(&Ctx<'_>, u64) -> JsResult<()>,
) {
    let mut app = host.app.clone();
    let entered = host.window.update(&mut app, |_, window, cx| {
        let Some(runtime) = ShellRuntime::global(cx) else {
            tracing::debug!("script task dropped: the shell runtime has already shut down");
            return;
        };

        let (guard, generation) = scope::enter(window, cx, ScopePhase::Task, owner);
        if let Err(error) = runtime.with_js(|ctx| body(ctx, generation)) {
            tracing::error!("error in script task: {error}");
        }
        drain_jobs(&runtime.js_runtime);
        drop(guard);
    });

    if let Err(error) = entered {
        tracing::debug!("script task dropped: {error}");
    }
}

// ---------------------------------------------------------------------------
// The task registry
// ---------------------------------------------------------------------------

type TaskId = u64;

thread_local! {
    /// Live tasks, so `cancel()` can reach work that has already been handed to
    /// the executor, and so shutdown can release the script functions they hold.
    /// A thread-local and not a field on the runtime because the VM, GPUI's
    /// `App` and every task here are all main-thread only.
    static TASKS: RefCell<HashMap<TaskId, Rc<TaskState>>> = RefCell::new(HashMap::new());
    static NEXT_TASK_ID: Cell<TaskId> = const { Cell::new(1) };
}

struct TaskState {
    id: TaskId,
    /// The JavaScript spelling of whatever created this task, for diagnostics.
    api: &'static str,
    /// `None` means the task was created with `owner: null` and outlives every
    /// view deliberately.
    owner: Option<WeakEntity<ScriptView>>,
    cancelled: Cell<bool>,
    done: Cell<bool>,
    /// The script function to resume with: a timer handler, or a promise's
    /// `resolve`. Held here rather than in the future so [`shutdown`] can
    /// release it while the QuickJS runtime is still alive.
    callback: RefCell<Option<Persistent<Function<'static>>>>,
}

/// Whether a task may run now, and the owner its scope belongs to.
enum Readiness {
    Ready(Option<Entity<ScriptView>>),
    Cancelled,
    OwnerGone,
}

impl TaskState {
    fn new(
        api: &'static str,
        owner: Option<WeakEntity<ScriptView>>,
        callback: Option<Persistent<Function<'static>>>,
    ) -> Self {
        Self {
            id: NEXT_TASK_ID.with(|next| {
                let id = next.get();
                next.set(id + 1);
                id
            }),
            api,
            owner,
            cancelled: Cell::new(false),
            done: Cell::new(false),
            callback: RefCell::new(callback),
        }
    }

    fn readiness(&self) -> Readiness {
        if self.cancelled.get() || self.done.get() {
            return Readiness::Cancelled;
        }
        match &self.owner {
            None => Readiness::Ready(None),
            Some(owner) => match owner.upgrade() {
                Some(view) => Readiness::Ready(Some(view)),
                None => Readiness::OwnerGone,
            },
        }
    }

    fn take_callback(&self) -> Option<Persistent<Function<'static>>> {
        self.callback.borrow_mut().take()
    }

    fn clone_callback(&self) -> Option<Persistent<Function<'static>>> {
        self.callback.borrow().clone()
    }
}

fn register(task: TaskState) -> Rc<TaskState> {
    let task = Rc::new(task);
    TASKS.with_borrow_mut(|tasks| tasks.insert(task.id, task.clone()));
    task
}

/// Marks a task finished and drops it from the registry.
///
/// Reaping here rather than sweeping later keeps a long-lived application from
/// accumulating one entry per elapsed timer.
fn finish(task: &Rc<TaskState>) {
    task.done.set(true);
    task.callback.replace(None);
    TASKS.with_borrow_mut(|tasks| tasks.remove(&task.id));
}

/// `handle.cancel()`.
///
/// A timer stops before its next tick and a sleep's promise is never settled.
/// JavaScript has no way to interrupt a promise chain that is already running,
/// so cancelling a `gpui.spawn` means the runtime stops adopting its outcome —
/// the chain itself runs on until its next host-owned await point, where the
/// owner and cancellation checks apply again.
fn cancel(id: TaskId) {
    let Some(task) = TASKS.with_borrow_mut(|tasks| tasks.remove(&id)) else {
        return;
    };
    tracing::trace!("cancelled {} (task {id})", task.api);
    task.cancelled.set(true);
    task.done.set(true);
    task.callback.replace(None);
}

/// `handle.is_done()`. A task the registry has forgotten has run or been
/// cancelled; either way there is nothing more to wait for.
fn is_done(id: TaskId) -> bool {
    TASKS.with_borrow(|tasks| tasks.get(&id).is_none_or(|task| task.done.get()))
}

// ---------------------------------------------------------------------------
// Arguments and diagnostics
// ---------------------------------------------------------------------------

/// Reads `opts.owner`, defaulting to the view whose call is in progress.
///
/// `null` is the deliberate opt-out for work that must outlive its view. Any
/// other view is refused rather than silently ignored: the engine can only
/// resolve the current view's script instance back to its entity, and a task
/// that quietly took the wrong owner would keep running after the panel it was
/// meant to follow had closed — exactly the bug ownership exists to prevent.
fn owner_from_options<'js>(
    ctx: &Ctx<'js>,
    opts: Option<&Value<'js>>,
) -> JsResult<Option<WeakEntity<ScriptView>>> {
    let current = scope::current_view();
    let default = || Ok(current.as_ref().map(Entity::downgrade));

    let Some(options) = opts.and_then(Value::as_object) else {
        return default();
    };
    let owner: Value = options.get("owner")?;
    if owner.is_undefined() {
        return default();
    }
    if owner.is_null() {
        return Ok(None);
    }

    let Some(owner) = owner.as_object() else {
        return Err(Exception::throw_type(
            ctx,
            "opts.owner must be a view instance, or null for a task that outlives every view",
        ));
    };

    let is_current = current
        .as_ref()
        .and_then(|view| scope::with_current_app(|app| view.read(app).object().clone()))
        .and_then(|object| object.restore(ctx).ok())
        .is_some_and(|object| &object == owner);

    if is_current {
        default()
    } else {
        Err(Exception::throw_type(
            ctx,
            "opts.owner must be the view that is running, or null; a task cannot yet be owned \
             by another view",
        ))
    }
}

fn duration(ctx: &Ctx<'_>, api: &str, ms: f64) -> JsResult<Duration> {
    if !ms.is_finite() || ms < 0.0 {
        return Err(Exception::throw_type(
            ctx,
            &format!("{api} expects a non-negative number of milliseconds, got {ms}"),
        ));
    }
    Ok(Duration::from_secs_f64(ms / 1000.0))
}

fn outside_host_call(ctx: &Ctx<'_>, api: &str) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        &format!(
            "{api} was called with no host call in progress. It needs the window and app of \
             a live call, so call it from render, from an event handler, or from a task the \
             scheduler resumed — not from a bare promise callback."
        ),
    )
}

/// Renders a thrown value the way its author would want to read it: message
/// first, then the script stack that produced it.
fn describe_value<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> String {
    if let Some(exception) = value.as_exception() {
        let message = exception.message().unwrap_or_else(|| "error".to_owned());
        return match exception.stack() {
            Some(stack) => format!("{message}\n{stack}"),
            None => message,
        };
    }

    if let Some(text) = value.as_string().and_then(|text| text.to_string().ok()) {
        return text;
    }

    match ctx.json_stringify(value.clone()) {
        Ok(Some(text)) => text.to_string().unwrap_or_else(|_| format!("{value:?}")),
        _ => format!("{value:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context as JsContext, Object};

    fn context() -> (JsRuntime, JsContext) {
        let runtime = JsRuntime::new().expect("runtime");
        let context = JsContext::full(&runtime).expect("context");
        (runtime, context)
    }

    /// The scheduler alone on `globalThis.gpui`, which is all these tests need.
    fn install_module(ctx: &Ctx<'_>) {
        let module = Object::new(ctx.clone()).expect("module");
        install(ctx, &module).expect("install");
        ctx.globals().set("gpui", module).expect("globals");
    }

    /// The failure this module exists to prevent: without a drain, a resolved
    /// promise's `.then` never runs and nothing says so.
    #[test]
    fn drain_jobs_runs_pending_promise_callbacks() {
        let (runtime, context) = context();

        context.with(|ctx| {
            ctx.eval::<Value, _>(
                r#"
                globalThis.resumed = false;
                globalThis.settle = null;
                globalThis.promise = new Promise((resolve) => { globalThis.settle = resolve; })
                    .then(() => { globalThis.resumed = true; });
                "#,
            )
            .expect("eval");
            ctx.eval::<Value, _>("settle()").expect("settle");

            assert!(
                !ctx.globals().get::<_, bool>("resumed").expect("resumed"),
                "a reaction must not run before the host drains the queue"
            );
        });

        drain_jobs(&runtime);

        context.with(|ctx| {
            assert!(
                ctx.globals().get::<_, bool>("resumed").expect("resumed"),
                "draining must run the queued reaction"
            );
        });
    }

    /// A drain with nothing queued is a no-op, not a hang.
    #[test]
    fn drain_jobs_returns_when_the_queue_is_empty() {
        let (runtime, context) = context();
        context.with(|ctx| {
            ctx.eval::<Value, _>("globalThis.x = 1").expect("eval");
        });
        drain_jobs(&runtime);
    }

    #[test]
    fn with_cx_outside_a_host_call_reports_clearly() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);

            let error = ctx
                .eval::<Value, _>("gpui.with_cx((cx) => cx)")
                .expect_err("with_cx must refuse to invent a context");
            describe(&ctx, error)
        });

        assert!(message.contains("gpui.with_cx(fn)"), "{message}");
        assert!(message.contains("no host call in progress"), "{message}");
    }

    /// Scheduling needs a window and an executor, both of which only exist
    /// during a host call. Saying so beats scheduling against a dead frame.
    #[test]
    fn sleep_outside_a_host_call_reports_clearly() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);

            let error = ctx
                .eval::<Value, _>("gpui.sleep(10)")
                .expect_err("sleep must refuse to schedule");
            describe(&ctx, error)
        });

        assert!(message.contains("gpui.sleep(ms)"), "{message}");
    }

    #[test]
    fn a_negative_delay_is_refused() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);

            let error = ctx
                .eval::<Value, _>("gpui.sleep(-1)")
                .expect_err("a negative delay must be refused");
            describe(&ctx, error)
        });

        assert!(message.contains("non-negative"), "{message}");
    }

    /// `gpui.spawn` needs no scope — an application starts work at module top
    /// level, before there is a window — and a rejection nobody handled must
    /// still settle the task rather than vanish.
    #[test]
    fn spawn_adopts_a_rejected_promise() {
        let (runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                "globalThis.handle = gpui.spawn(async () => { throw new Error(\"boom\"); });",
            )
            .expect("spawn");

            assert!(
                !ctx.eval::<bool, _>("handle.is_done()").expect("is_done"),
                "an adopted promise is still pending until the queue is drained"
            );
        });

        drain_jobs(&runtime);

        context.with(|ctx| {
            assert!(
                ctx.eval::<bool, _>("handle.is_done()").expect("is_done"),
                "the rejection handler must settle the task"
            );
        });
    }

    #[test]
    fn spawn_settles_on_a_resolved_promise() {
        let (runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                r#"
                globalThis.ran = false;
                globalThis.handle = gpui.spawn(async () => { globalThis.ran = true; });
                "#,
            )
            .expect("spawn");

            assert!(
                ctx.globals().get::<_, bool>("ran").expect("ran"),
                "the body runs synchronously up to its first await"
            );
            assert!(!ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });

        drain_jobs(&runtime);

        context.with(|ctx| {
            assert!(ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });
    }

    /// A synchronous throw is reported the same way a rejection is, and does not
    /// escape into the caller as a live exception.
    #[test]
    fn spawn_survives_a_body_that_throws_at_once() {
        let (_runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                "globalThis.handle = gpui.spawn(() => { throw new Error(\"boom\"); });",
            )
            .expect("spawn must absorb the throw");

            assert!(ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });
    }

    #[test]
    fn a_cancelled_spawn_reports_itself_done() {
        let (_runtime, context) = context();

        context.with(|ctx| {
            install_module(&ctx);
            ctx.eval::<Value, _>(
                "globalThis.handle = gpui.spawn(async () => { await Promise.resolve(); });",
            )
            .expect("spawn");

            assert!(!ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
            ctx.eval::<Value, _>("handle.cancel()").expect("cancel");
            assert!(ctx.eval::<bool, _>("handle.is_done()").expect("is_done"));
        });
    }

    /// An owner that is not the view in progress is refused rather than quietly
    /// downgraded to "no owner", which would defeat the point of ownership.
    #[test]
    fn an_unknown_owner_is_refused() {
        let (_runtime, context) = context();

        let message = context.with(|ctx| {
            install_module(&ctx);
            let error = ctx
                .eval::<Value, _>("gpui.spawn(async () => {}, { owner: {} })")
                .expect_err("an unrelated owner must be refused");
            describe(&ctx, error)
        });

        assert!(message.contains("opts.owner"), "{message}");
    }

    #[test]
    fn a_cancelled_task_never_becomes_ready() {
        let task = register(TaskState::new("test", None, None));
        assert!(matches!(task.readiness(), Readiness::Ready(None)));
        assert!(!is_done(task.id));

        cancel(task.id);

        assert!(
            matches!(task.readiness(), Readiness::Cancelled),
            "a cancelled timer must not fire on its next tick"
        );
        assert!(is_done(task.id), "a cancelled task reports itself done");
        assert!(
            TASKS.with_borrow(|tasks| !tasks.contains_key(&task.id)),
            "cancelling reaps the registry entry"
        );
    }

    #[test]
    fn a_finished_task_is_reaped() {
        let task = register(TaskState::new("test", None, None));
        finish(&task);

        assert!(is_done(task.id));
        assert!(TASKS.with_borrow(|tasks| !tasks.contains_key(&task.id)));
        // The handle keeps answering after the entry is gone.
        assert!(is_done(task.id + 10_000));
    }

    /// The rule from §12.3: work whose panel has closed must not run. Note it is
    /// skipped, not cancelled — nobody called `cancel`, the owner simply left.
    #[gpui::test]
    async fn a_task_whose_owner_is_gone_is_skipped(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;

        let runtime = ShellRuntime::new().expect("runtime");
        let object = runtime
            .context
            .with(|ctx| Persistent::save(&ctx, Object::new(ctx.clone()).expect("object")));

        let view = cx.update(|cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
        let task = register(TaskState::new("test", Some(view.downgrade()), None));
        assert!(matches!(task.readiness(), Readiness::Ready(Some(_))));

        drop(view);
        cx.update(|_| {});

        assert!(matches!(task.readiness(), Readiness::OwnerGone));
        finish(&task);

        // The QuickJS runtime is leaked on purpose: `ScriptView` holds a
        // `Persistent`, test teardown order is not ours to choose, and a
        // `Persistent` released after its runtime aborts the process.
        std::mem::forget(runtime);
    }
}
