//! Host context that is valid only for the duration of one Rust → script call.
//!
//! GPUI hands out `&mut Window` and `&mut App` as borrows. Script userdata outlives
//! any borrow, so a `cx` captured during one call and used from a later timer
//! would point at a dead stack frame. [`CallScope`] turns "am I inside a legal
//! host call?" into a runtime-checkable fact: every entry point pushes a frame
//! with a fresh generation, and the script-side `cx` only carries that generation.
//! A frame may additionally [`adopt`] one earlier call's generation, which widens
//! the set of `cx` values it answers for without changing where their pointers
//! come from.
//!
//! # Safety
//!
//! The raw pointers below are sound because:
//!
//! - the script VM and GPUI's `App` are both main-thread only, so no other thread
//!   can observe the stack;
//! - frames are strictly last-in-first-out, enforced by [`CallScopeGuard`];
//! - a frame's pointers are only reachable while its guard is alive;
//! - [`with_current_app`], [`with_current`] and [`with_context`] share one
//!   runtime borrow guard, so a host callback cannot re-enter one of them while
//!   another `&mut App` / `&mut Window` reconstructed from the frame is live.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use gpui::{App, Entity, Window};

use crate::{
    engine::ShellRuntime, policy::Policy, runtime::ApplicationGeneration, view::ScriptView,
};

/// What the current host call is allowed to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScopePhase {
    /// Building an element tree. May read state and register callbacks.
    Render,
    /// Handling an event. May mutate state, notify, spawn.
    Event,
    /// Resuming an async task. Same powers as [`ScopePhase::Event`].
    Task,
    /// Inside GPUI layout/prepaint, rendering one virtualized item.
    Layout,
}

impl ScopePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            ScopePhase::Render => "render",
            ScopePhase::Event => "event",
            ScopePhase::Task => "task",
            ScopePhase::Layout => "layout",
        }
    }

    /// Whether this phase may request a re-render.
    pub fn allows_notify(self) -> bool {
        matches!(self, ScopePhase::Event | ScopePhase::Task)
    }
}

struct Frame {
    window: *mut Window,
    app: *mut App,
    phase: ScopePhase,
    generation: u64,
    /// An earlier call this frame also answers for, if any. See [`adopt`].
    adopted: Option<u64>,
    view: Option<Entity<ScriptView>>,
    /// Whose authority this code runs under.
    ///
    /// On the frame rather than on the thread or the runtime, because that is
    /// the only place that survives an `await`: a continuation resuming later
    /// brings its own frame back, and nothing can have been swapped underneath
    /// it. See [`crate::policy`] for why the alternatives cannot be made
    /// correct.
    policy: Rc<Policy>,
    /// Which evaluated incarnation of the application owns this call.
    application: Option<Rc<ApplicationGeneration>>,
    /// The VM this call entered. Async work inherits this exact runtime.
    runtime: Weak<ShellRuntime>,
}

thread_local! {
    static STACK: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static NEXT_GENERATION: Cell<u64> = const { Cell::new(1) };
    static HOST_CONTEXT_BORROWED: Cell<bool> = const { Cell::new(false) };
}

struct HostContextBorrow;

impl HostContextBorrow {
    fn acquire() -> Option<Self> {
        HOST_CONTEXT_BORROWED.with(|borrowed| {
            if borrowed.replace(true) {
                None
            } else {
                Some(Self)
            }
        })
    }
}

impl Drop for HostContextBorrow {
    fn drop(&mut self) {
        HOST_CONTEXT_BORROWED.with(|borrowed| {
            let was_borrowed = borrowed.replace(false);
            debug_assert!(was_borrowed);
        });
    }
}

/// Pops the frame it owns when dropped.
pub struct CallScopeGuard {
    _private: (),
}

impl Drop for CallScopeGuard {
    fn drop(&mut self) {
        STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

/// Opens a scope. The returned generation is what the script-side `cx` carries.
pub fn enter(
    window: &mut Window,
    app: &mut App,
    phase: ScopePhase,
    view: Option<Entity<ScriptView>>,
) -> (CallScopeGuard, u64) {
    // A view carries the policy its script runs under; without one — loading a
    // module, constructing a view — the call inherits whatever the enclosing
    // frame was running under, and the outermost such call falls back to the
    // host's default.
    let policy = view
        .as_ref()
        .and_then(|view| with_current_app(|cx| view.read(cx).policy()))
        .or_else(current_policy)
        .unwrap_or_else(crate::policy::default);

    let runtime = view
        .as_ref()
        .and_then(|view| with_current_app(|cx| view.read(cx).runtime()))
        .or_else(current_runtime)
        .or_else(|| ShellRuntime::global(app));

    let application = view
        .as_ref()
        .and_then(|view| with_current_app(|cx| view.read(cx).application_generation()))
        .flatten()
        .or_else(current_application_generation);
    enter_with_runtime_opt(
        window,
        app,
        phase,
        view,
        policy,
        runtime.as_ref(),
        application,
    )
}

/// Opens a scope for a runtime that the caller already knows.
#[allow(dead_code)] // Used by the optional overlay engine and its tests.
pub fn enter_runtime(
    runtime: &Rc<ShellRuntime>,
    window: &mut Window,
    app: &mut App,
    phase: ScopePhase,
    view: Option<Entity<ScriptView>>,
) -> (CallScopeGuard, u64) {
    let policy = view
        .as_ref()
        .map(|view| view.read(app).policy())
        .or_else(current_policy)
        .unwrap_or_else(crate::policy::default);
    enter_with_runtime(runtime, window, app, phase, view, policy)
}

/// Opens a scope under a policy the caller names.
///
/// The entry points with no view yet use this: "under whose authority is this
/// code being loaded" is a question the caller answers rather than inherits.
pub fn enter_with_runtime(
    runtime: &Rc<ShellRuntime>,
    window: &mut Window,
    app: &mut App,
    phase: ScopePhase,
    view: Option<Entity<ScriptView>>,
    policy: Rc<Policy>,
) -> (CallScopeGuard, u64) {
    let application = view
        .as_ref()
        .and_then(|view| view.read(app).application_generation())
        .or_else(current_application_generation);
    enter_with_runtime_opt(window, app, phase, view, policy, Some(runtime), application)
}

/// Opens a scope owned by an explicitly selected application incarnation.
pub(crate) fn enter_with_application(
    runtime: &Rc<ShellRuntime>,
    window: &mut Window,
    app: &mut App,
    phase: ScopePhase,
    view: Option<Entity<ScriptView>>,
    policy: Rc<Policy>,
    application: Option<Rc<ApplicationGeneration>>,
) -> (CallScopeGuard, u64) {
    enter_with_runtime_opt(window, app, phase, view, policy, Some(runtime), application)
}

/// Rebinds the current host call to a newly evaluated application.
///
/// Module evaluation happens below an already-open host scope but before a
/// view exists. Duplicating the frame with no view prevents new top-level work
/// from inheriting the old view during reload.
pub(crate) fn enter_application(
    application: Rc<ApplicationGeneration>,
) -> Option<(CallScopeGuard, u64)> {
    let frame = STACK.with(|stack| {
        let stack = stack.borrow();
        let frame = stack.last()?;
        Some((
            frame.window,
            frame.app,
            frame.phase,
            frame.policy.clone(),
            frame.runtime.clone(),
        ))
    })?;
    let generation = NEXT_GENERATION.with(|next| {
        let value = next.get();
        next.set(value + 1);
        value
    });
    STACK.with(|stack| {
        stack.borrow_mut().push(Frame {
            window: frame.0,
            app: frame.1,
            phase: frame.2,
            generation,
            adopted: None,
            view: None,
            policy: frame.3,
            application: Some(application),
            runtime: frame.4,
        });
    });
    Some((CallScopeGuard { _private: () }, generation))
}

fn enter_with_runtime_opt(
    window: &mut Window,
    app: &mut App,
    phase: ScopePhase,
    view: Option<Entity<ScriptView>>,
    policy: Rc<Policy>,
    runtime: Option<&Rc<ShellRuntime>>,
    application: Option<Rc<ApplicationGeneration>>,
) -> (CallScopeGuard, u64) {
    let generation = NEXT_GENERATION.with(|next| {
        let value = next.get();
        next.set(value + 1);
        value
    });

    STACK.with(|stack| {
        stack.borrow_mut().push(Frame {
            window: window as *mut Window,
            app: app as *mut App,
            phase,
            generation,
            adopted: None,
            view,
            policy,
            application,
            runtime: runtime.map_or_else(Weak::new, Rc::downgrade),
        })
    });

    (CallScopeGuard { _private: () }, generation)
}

/// Lets the innermost frame answer for an earlier call's `cx` as well as its own.
///
/// One caller: the frame a virtualized list's item renderer runs in. That
/// renderer is a closure the script wrote inside `render(cx)`, and the `cx` it
/// closed over is the render's — but GPUI calls it from inside layout, long
/// after the render frame was popped, so every helper the rows call would
/// otherwise have to be re-plumbed for lists alone. Naming the render's
/// generation here keeps that one `cx` working and leaves every other stale one
/// refused; the pointers a member reaches are still the live frame's.
///
/// `None` is a no-op, which is what a callback registered outside any host call
/// deserves.
pub(crate) fn adopt(generation: Option<u64>) {
    let Some(generation) = generation else {
        return;
    };
    STACK.with(|stack| {
        if let Some(frame) = stack.borrow_mut().last_mut() {
            frame.adopted = Some(generation);
        }
    });
}

pub fn current_runtime() -> Option<Rc<ShellRuntime>> {
    STACK.with(|stack| {
        stack
            .borrow()
            .last()
            .and_then(|frame| frame.runtime.upgrade())
    })
}

/// The generation of the innermost scope, if any.
pub fn current_generation() -> Option<u64> {
    STACK.with(|stack| stack.borrow().last().map(|frame| frame.generation))
}

/// The phase of the innermost scope, if any.
pub fn current_phase() -> Option<ScopePhase> {
    STACK.with(|stack| stack.borrow().last().map(|frame| frame.phase))
}

/// Runs `f` with the innermost scope's `App`, whatever its generation.
///
/// Used by conversions that need to read globals (theme tokens) while a script
/// call is in progress. Returns `None` outside any scope.
pub fn with_current_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    let app = STACK.with(|stack| stack.borrow().last().map(|frame| frame.app))?;
    let _borrow = HostContextBorrow::acquire()?;
    // SAFETY: see the module header.
    Some(f(unsafe { &mut *app }))
}

/// Runs `f` with the innermost scope's `Window` and `App`.
///
/// Creating a retained entity — an input's state, a tree's state — needs both,
/// and it happens while script code is running rather than at a known point in
/// the host, so the context comes from the scope stack rather than being
/// threaded through. Returns `None` outside any scope, which is the honest
/// answer for "the script asked for this from nowhere".
pub fn with_current<R>(f: impl FnOnce(&mut Window, &mut App) -> R) -> Option<R> {
    let pointers =
        STACK.with(|stack| stack.borrow().last().map(|frame| (frame.window, frame.app)))?;
    let _borrow = HostContextBorrow::acquire()?;
    // SAFETY: see the module header.
    Some(f(unsafe { &mut *pointers.0 }, unsafe { &mut *pointers.1 }))
}

/// The policy the innermost scope runs under, if there is one.
///
/// Every read of a grant goes through here rather than through a thread-local,
/// which is what makes two plugins in one runtime hold two different grants at
/// the same time.
pub fn current_policy() -> Option<Rc<Policy>> {
    STACK.with(|stack| stack.borrow().last().map(|frame| frame.policy.clone()))
}

/// The policy in force, falling back to the host's default outside any call.
pub fn policy() -> Rc<Policy> {
    current_policy().unwrap_or_else(crate::policy::default)
}

/// The view the innermost scope belongs to, if any.
pub fn current_view() -> Option<Entity<ScriptView>> {
    STACK.with(|stack| stack.borrow().last().and_then(|frame| frame.view.clone()))
}

/// The evaluated application incarnation the innermost call belongs to.
pub(crate) fn current_application_generation() -> Option<Rc<ApplicationGeneration>> {
    STACK.with(|stack| {
        stack
            .borrow()
            .last()
            .and_then(|frame| frame.application.clone())
    })
}

/// Runs `f` with the innermost scope's context, if `generation` still names a
/// live call — the innermost frame's own, or one that frame [`adopt`]ed.
///
/// A stale generation is a programming error in the script, not a host bug, so
/// it produces a descriptive error rather than a panic.
pub fn with_context<R>(
    generation: u64,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> Result<R, StaleContext> {
    let pointers = STACK.with(|stack| {
        stack
            .borrow()
            .last()
            .filter(|frame| frame.generation == generation || frame.adopted == Some(generation))
            .map(|frame| (frame.window, frame.app))
    });

    match (pointers, HostContextBorrow::acquire()) {
        // SAFETY: see the module header. The frame is the innermost one, its
        // guard is therefore still alive, and nothing else can be holding these
        // borrows on this thread while the script is running.
        (Some((window, app)), Some(_borrow)) => {
            Ok(f(unsafe { &mut *window }, unsafe { &mut *app }))
        }
        _ => Err(StaleContext),
    }
}

/// The script used a `cx` that belongs to a call which has already returned.
#[derive(Debug)]
pub struct StaleContext;

impl std::fmt::Display for StaleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "cx is no longer valid: it was captured during an earlier call and used later. \
             Use cx.spawn or take cx from the callback arguments instead.",
        )
    }
}

impl std::error::Error for StaleContext {}

#[cfg(test)]
mod tests {
    use gpui::{Render, TestAppContext, VisualTestContext};

    use super::*;

    struct Empty;

    impl Render for Empty {
        fn render(
            &mut self,
            _: &mut Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn host_context_access_cannot_reenter(cx: &mut TestAppContext) {
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window, cx);

        context.update(|window, cx| {
            let (_scope, generation) = enter(window, cx, ScopePhase::Task, None);
            let nested = with_current(|_, _| {
                (
                    with_current_app(|_| ()),
                    with_current(|_, _| ()),
                    with_context(generation, |_, _| ()),
                )
            });
            let Some((app, window_and_app, context)) = nested else {
                panic!("the outer host-context borrow should succeed");
            };
            assert!(app.is_none());
            assert!(window_and_app.is_none());
            assert!(context.is_err());
            assert!(
                with_current_app(|_| ()).is_some(),
                "dropping the outer borrow must allow the next access"
            );
        });
    }
}
