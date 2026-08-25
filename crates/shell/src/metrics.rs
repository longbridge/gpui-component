//! What the runtime is actually spending, in numbers a test can assert on.
//!
//! The central claim of this runtime is that script cost follows application
//! activity rather than frame rate (`snapshot.rs`). A claim that cannot be
//! observed cannot be regression-tested, so it is a counter rather than a
//! comment: `tests/snapshot.rs` renders a clean view repeatedly and asserts that
//! [`RuntimeMetrics::script_renders`] has not moved, and the shell story shows
//! both counters live while a feed drives the view.
//!
//! Two counters, and the gap between them is the whole point:
//!
//! ```text
//! script_renders    ── follows cx.notify(), reloads, theme changes
//! materializations  ── follows GPUI frames
//! ```
//!
//! Timing uses `instant`, which is `std::time::Instant` everywhere except wasm,
//! where `std::time::Instant::now` panics outright.

use std::{cell::Cell, time::Duration};

/// A reading of the runtime's counters.
///
/// Values are a snapshot taken at the moment [`Metrics::read`] was called;
/// nothing here keeps updating behind the caller's back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeMetrics {
    script_renders: u64,
    script_render_time: Duration,
    materializations: u64,
    materialize_time: Duration,
}

impl RuntimeMetrics {
    /// How many times script `render` has been entered.
    pub fn script_renders(&self) -> u64 {
        self.script_renders
    }

    /// Total time spent inside script `render`, including argument conversion
    /// and description recording.
    pub fn script_render_time(&self) -> Duration {
        self.script_render_time
    }

    /// How many times a snapshot has been turned into GPUI elements. This one
    /// follows frames.
    pub fn materializations(&self) -> u64 {
        self.materializations
    }

    /// Total time spent materializing, which is the part of the runtime that
    /// belongs to the frame budget.
    pub fn materialize_time(&self) -> Duration {
        self.materialize_time
    }

    pub fn mean_script_render(&self) -> Duration {
        mean(self.script_render_time, self.script_renders)
    }

    pub fn mean_materialize(&self) -> Duration {
        mean(self.materialize_time, self.materializations)
    }

    /// What this reading gained over an earlier one.
    ///
    /// Rates are what a live readout wants — "script renders in the last
    /// second" says something "script renders since start-up" does not — and a
    /// difference of two readings is the honest way to get one without the
    /// runtime having to know what a second is.
    pub fn since(&self, earlier: &Self) -> Self {
        Self {
            script_renders: self.script_renders.saturating_sub(earlier.script_renders),
            script_render_time: self
                .script_render_time
                .saturating_sub(earlier.script_render_time),
            materializations: self
                .materializations
                .saturating_sub(earlier.materializations),
            materialize_time: self
                .materialize_time
                .saturating_sub(earlier.materialize_time),
        }
    }
}

fn mean(total: Duration, count: u64) -> Duration {
    match u32::try_from(count) {
        Ok(0) | Err(_) => Duration::ZERO,
        Ok(count) => total / count,
    }
}

/// The live counters, owned by the runtime.
///
/// `Cell` rather than an atomic because the VM and GPUI's `App` are both
/// main-thread only, and rather than a `RefCell` because a counter that could
/// panic on a re-entrant borrow would be a poor thing to put on the render path.
#[derive(Default)]
pub struct Metrics {
    script_renders: Cell<u64>,
    script_render_nanos: Cell<u64>,
    materializations: Cell<u64>,
    materialize_nanos: Cell<u64>,
}

impl Metrics {
    /// Times `build`, which is one entry into script `render`.
    pub fn time_script_render<R>(&self, build: impl FnOnce() -> R) -> R {
        let started = instant::Instant::now();
        let result = build();
        self.script_renders.set(self.script_renders.get() + 1);
        self.script_render_nanos
            .set(self.script_render_nanos.get() + elapsed_nanos(started));
        result
    }

    /// Times `build`, which is one materialization of a snapshot.
    pub fn time_materialize<R>(&self, build: impl FnOnce() -> R) -> R {
        let started = instant::Instant::now();
        let result = build();
        self.materializations.set(self.materializations.get() + 1);
        self.materialize_nanos
            .set(self.materialize_nanos.get() + elapsed_nanos(started));
        result
    }

    pub fn read(&self) -> RuntimeMetrics {
        RuntimeMetrics {
            script_renders: self.script_renders.get(),
            script_render_time: Duration::from_nanos(self.script_render_nanos.get()),
            materializations: self.materializations.get(),
            materialize_time: Duration::from_nanos(self.materialize_nanos.get()),
        }
    }

    /// Zeroes every counter, so a measurement can start from a known point
    /// rather than from whatever start-up happened to do.
    pub fn reset(&self) {
        self.script_renders.set(0);
        self.script_render_nanos.set(0);
        self.materializations.set(0);
        self.materialize_nanos.set(0);
    }
}

fn elapsed_nanos(started: instant::Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
