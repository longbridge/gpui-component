//! A realtime performance HUD for GPUI applications: frames per second, a
//! rolling frame time chart, and this process' CPU and memory usage.
//!
//! Frame data comes from GPUI's own frame trace
//! ([`gpui::FrameTimingCollector`]), so the numbers are what the framework
//! actually spent in `Window::draw` rather than an approximation measured from
//! the outside.
//!
//! Mount it once in the window's root view:
//!
//! ```no_run
//! # use gpui::*;
//! # use gpui_perf::fps_monitor;
//! # struct Example;
//! # impl Render for Example {
//! fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!     div()
//!         .relative()
//!         .size_full()
//!         .child("your app")
//!         .child(fps_monitor(window, cx))
//! }
//! # }
//! ```
//!
//! and switch it on and off from anywhere:
//!
//! ```no_run
//! # use gpui::*;
//! # fn example(window: &mut Window, cx: &mut App) {
//! gpui_perf::toggle_fps(window, cx);
//! # }
//! ```
//!
//! Neither call takes options. Anything else — a different corner, frame
//! budget, palette, or an embedded rather than overlaid HUD — is built by
//! composing the two pieces they use, [`FpsMonitor`] and [`FpsOverlay`].
//!
//! This crate depends only on `gpui`, so it can be used from any GPUI
//! application.

mod monitor;
mod overlay;
mod sampler;
mod style;

pub use monitor::FpsMonitor;
pub use overlay::FpsOverlay;

use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use gpui::{
    AnyElement, App, AppContext as _, Empty, Entity, Global, IntoElement as _, Window, WindowId,
};

/// Mounts the performance HUD in the top right of its parent.
///
/// Call this once from the window's root view. Nothing is drawn until the HUD
/// is switched on with [`toggle_fps`], so the mount point can be left in place
/// permanently:
///
/// ```no_run
/// # use gpui::*;
/// # use gpui_perf::fps_monitor;
/// # struct Example;
/// # impl Render for Example {
/// fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
///     div()
///         // The HUD positions itself absolutely, so the parent must be relative.
///         .relative()
///         .size_full()
///         .child("your app")
///         .child(fps_monitor(window, cx))
/// }
/// # }
/// ```
///
/// GPUI draws only what the element tree produces — `Window`'s paint methods
/// assert they are called from an element's paint phase — so there is no way to
/// put the HUD on screen without a mount point somewhere. This is the smallest
/// one: a single line, written once, that costs nothing while hidden.
///
/// The monitor is created on first display and reused afterwards, one per
/// window. Build a [`FpsOverlay`] over your own [`FpsMonitor`] to place it
/// somewhere else, or to change the frame budget or palette.
pub fn fps_monitor(window: &mut Window, cx: &mut App) -> AnyElement {
    if !fps_visible(window, cx) {
        return Empty.into_any_element();
    }

    let window_id = window.window_handle().window_id();
    let existing = cx
        .try_global::<Monitors>()
        .and_then(|state| state.monitors.get(&window_id).cloned());
    let monitor = match existing {
        Some(monitor) => monitor,
        None => {
            let monitor = cx.new(|cx| FpsMonitor::new(window, cx));
            cx.default_global::<Monitors>()
                .monitors
                .insert(window_id, monitor.clone());
            monitor
        }
    };

    // Returned bare, not wrapped: taffy anchors an absolutely positioned node
    // to its direct parent rather than to the nearest non-static ancestor the
    // way CSS does, so any wrapper here would become the positioning context
    // and strand the HUD in a zero-sized box.
    FpsOverlay::new(&monitor).into_any_element()
}

/// Shows the HUD if it is hidden and hides it if it is shown.
///
/// Takes effect wherever [`fps_monitor`] was mounted for this window.
pub fn toggle_fps(window: &mut Window, cx: &mut App) {
    let window_id = window.window_handle().window_id();
    let state = cx.default_global::<Monitors>();
    if !state.visible.remove(&window_id) {
        state.visible.insert(window_id);
    }
    window.refresh();
}

/// Whether the HUD is currently shown for this window.
pub fn fps_visible(window: &Window, cx: &App) -> bool {
    let window_id = window.window_handle().window_id();
    cx.try_global::<Monitors>()
        .is_some_and(|state| state.visible.contains(&window_id))
}

/// Per-window HUD state.
///
/// Entries outlive their window; the leak is one small entity per window that
/// ever showed the HUD, which is not worth tracking window closes for.
#[derive(Default)]
struct Monitors {
    monitors: HashMap<WindowId, Entity<FpsMonitor>>,
    visible: HashSet<WindowId>,
}

impl Global for Monitors {}

struct TraceState {
    /// Number of live [`FrameTraceGuard`]s.
    refs: usize,
    /// Whether frame tracing was already on when the first guard was taken,
    /// meaning the host application owns the switch and we must leave it alone.
    owned_by_host: bool,
}

static TRACE_STATE: Mutex<TraceState> = Mutex::new(TraceState {
    refs: 0,
    owned_by_host: false,
});

/// Keeps GPUI's frame trace enabled for as long as it is alive.
///
/// [`gpui::set_frame_trace_enabled`] is a process-wide switch, and turning it
/// off clears the recorded buffer. A monitor therefore must not disable it
/// while another monitor — or the host application's own profiling — still
/// depends on it, so guards are reference counted and the switch is only
/// restored by the last one. If tracing was already on before the first guard,
/// it is never turned off.
pub(crate) struct FrameTraceGuard {
    _private: (),
}

impl FrameTraceGuard {
    /// Enables frame tracing if it isn't already on.
    pub(crate) fn acquire() -> Self {
        if let Ok(mut state) = TRACE_STATE.lock() {
            if state.refs == 0 {
                // Returns false when the value was already `true`, which means
                // somebody else turned tracing on and owns restoring it.
                state.owned_by_host = !gpui::set_frame_trace_enabled(true);
            }
            state.refs += 1;
        }
        Self { _private: () }
    }
}

impl Drop for FrameTraceGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = TRACE_STATE.lock() {
            state.refs = state.refs.saturating_sub(1);
            if state.refs == 0 && !state.owned_by_host {
                gpui::set_frame_trace_enabled(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_guards_keep_tracing_on_until_the_last_one_drops() {
        let outer = FrameTraceGuard::acquire();
        let inner = FrameTraceGuard::acquire();
        assert!(gpui::frame_trace_enabled());

        drop(inner);
        assert!(
            gpui::frame_trace_enabled(),
            "the outer guard still needs the trace"
        );

        drop(outer);
        assert!(!gpui::frame_trace_enabled());
    }
}
