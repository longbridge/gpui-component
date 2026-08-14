//! A realtime performance HUD for GPUI applications: frames per second, a
//! rolling frame time chart, and this process' CPU and memory usage.
//!
//! Frame data comes from GPUI's own frame trace
//! ([`gpui::FrameTimingCollector`]), so the numbers are what the framework
//! actually spent in `Window::draw` rather than an approximation measured from
//! the outside.
//!
//! The normal way to use this is one call, which decides only *where* the HUD
//! sits. Everything else — creating the monitor, keeping it alive across
//! frames, enabling the frame trace, sampling CPU and memory — is handled
//! internally:
//!
//! ```no_run
//! # use gpui::*;
//! # use gpui_perf::fps_monitor;
//! # struct Example;
//! # impl Render for Example {
//! fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!     div()
//!         // The HUD positions itself absolutely, so the parent must be relative.
//!         .relative()
//!         .size_full()
//!         .child("your app")
//!         .child(fps_monitor(Anchor::TopRight, window, cx))
//! }
//! # }
//! ```
//!
//! [`FpsMonitor`], [`PerfOverlay`] and [`PerfStyle`] are available for the
//! cases that call for a custom frame budget, palette, or placement.
//!
//! This crate depends only on `gpui`, so it can be used from any GPUI
//! application.

mod monitor;
mod overlay;
mod sampler;
mod style;

pub use monitor::FpsMonitor;
pub use overlay::PerfOverlay;
pub use sampler::{FrameSample, ResourceSample};
pub use style::PerfStyle;

use std::{collections::HashMap, sync::Mutex};

use gpui::{Anchor, App, AppContext as _, Entity, Global, IntoElement, Window, WindowId};

/// Renders the performance HUD pinned to `anchor` of the current window.
///
/// The monitor backing the HUD is created on first use and reused afterwards,
/// one per window, so this can be called straight from `render` every frame.
///
/// The parent element must be `relative()`, since the HUD positions itself
/// absolutely. Call this at most once per window — a second call renders the
/// same monitor twice.
pub fn fps_monitor(anchor: Anchor, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let window_id = window.window_handle().window_id();

    let existing = cx
        .try_global::<Monitors>()
        .and_then(|monitors| monitors.0.get(&window_id).cloned());
    let monitor = match existing {
        Some(monitor) => monitor,
        None => {
            let monitor = cx.new(|cx| FpsMonitor::new(window, cx));
            cx.default_global::<Monitors>()
                .0
                .insert(window_id, monitor.clone());
            monitor
        }
    };

    PerfOverlay::new(&monitor).anchor(anchor)
}

/// The monitor [`fps_monitor`] reuses for each window.
///
/// Entries outlive their window; the leak is one small entity per window that
/// ever showed the HUD, which is not worth tracking window closes for.
#[derive(Default)]
struct Monitors(HashMap<WindowId, Entity<FpsMonitor>>);

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
pub struct FrameTraceGuard {
    _private: (),
}

impl FrameTraceGuard {
    /// Enables frame tracing if it isn't already on.
    pub fn acquire() -> Self {
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
