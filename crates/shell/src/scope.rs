//! Host context that is valid only for the duration of one Rust → Lua call.
//!
//! GPUI hands out `&mut Window` and `&mut App` as borrows. Lua userdata outlives
//! any borrow, so a `cx` captured during one call and used from a later timer
//! would point at a dead stack frame. [`CallScope`] turns "am I inside a legal
//! host call?" into a runtime-checkable fact: every entry point pushes a frame
//! with a fresh generation, and the Lua-side `cx` only carries that generation.
//!
//! # Safety
//!
//! The raw pointers below are sound because:
//!
//! - the Lua VM and GPUI's `App` are both main-thread only, so no other thread
//!   can observe the stack;
//! - frames are strictly last-in-first-out, enforced by [`CallScopeGuard`];
//! - a frame's pointers are only reachable while its guard is alive.

use std::cell::{Cell, RefCell};

use gpui::{App, Entity, Window};

use crate::view::ScriptView;

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
    view: Option<Entity<ScriptView>>,
}

thread_local! {
    static STACK: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static NEXT_GENERATION: Cell<u64> = const { Cell::new(1) };
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

/// Opens a scope. The returned generation is what the Lua-side `cx` carries.
pub fn enter(
    window: &mut Window,
    app: &mut App,
    phase: ScopePhase,
    view: Option<Entity<ScriptView>>,
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
            view,
        })
    });

    (CallScopeGuard { _private: () }, generation)
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
/// Used by conversions that need to read globals (theme tokens) while a Lua
/// call is in progress. Returns `None` outside any scope.
pub fn with_current_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    let app = STACK.with(|stack| stack.borrow().last().map(|frame| frame.app));
    // SAFETY: see the module header.
    app.map(|app| f(unsafe { &mut *app }))
}

/// Runs `f` with the innermost scope's `Window` and `App`.
///
/// Creating a retained entity — an input's state, a tree's state — needs both,
/// and it happens while script code is running rather than at a known point in
/// the host, so the context comes from the scope stack rather than being
/// threaded through. Returns `None` outside any scope, which is the honest
/// answer for "the script asked for this from nowhere".
pub fn with_current<R>(f: impl FnOnce(&mut Window, &mut App) -> R) -> Option<R> {
    let pointers = STACK.with(|stack| stack.borrow().last().map(|frame| (frame.window, frame.app)));
    // SAFETY: see the module header.
    pointers.map(|(window, app)| f(unsafe { &mut *window }, unsafe { &mut *app }))
}

/// The view the innermost scope belongs to, if any.
pub fn current_view() -> Option<Entity<ScriptView>> {
    STACK.with(|stack| stack.borrow().last().and_then(|frame| frame.view.clone()))
}

/// Runs `f` with the innermost scope's context, if `generation` is still current.
///
/// A stale generation is a programming error in the Lua code, not a host bug, so
/// it produces a descriptive error rather than a panic.
pub fn with_context<R>(
    generation: u64,
    f: impl FnOnce(&mut Window, &mut App) -> R,
) -> Result<R, StaleContext> {
    let pointers = STACK.with(|stack| {
        stack
            .borrow()
            .last()
            .filter(|frame| frame.generation == generation)
            .map(|frame| (frame.window, frame.app))
    });

    match pointers {
        // SAFETY: see the module header. The frame is the innermost one, its
        // guard is therefore still alive, and nothing else can be holding these
        // borrows on this thread while Lua is running.
        Some((window, app)) => Ok(f(unsafe { &mut *window }, unsafe { &mut *app })),
        None => Err(StaleContext),
    }
}

/// The Lua code used a `cx` that belongs to a call which has already returned.
#[derive(Debug)]
pub struct StaleContext;

impl std::fmt::Display for StaleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "cx is no longer valid: it was captured during an earlier call and used later. \
             Use gpui.spawn or take cx from the callback arguments instead.",
        )
    }
}

impl std::error::Error for StaleContext {}
