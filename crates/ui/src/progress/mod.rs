mod progress;
mod progress_circle;

pub use progress::Progress;
pub use progress_circle::ProgressCircle;

use std::cell::Cell;

/// Shared state for progress components.
///
/// Tracks the animation "from" value (`value`) and the latest target
/// (`target`). `target` uses `Cell` so it can be updated immediately
/// during render without triggering a re-render notification.
pub(crate) struct ProgressState {
    /// The "from" value for the current animation; updated by the async
    /// timer once the animation completes.
    pub(crate) value: f32,
    /// The latest animation target, updated immediately via interior
    /// mutability so stale timers always read the up-to-date target.
    target: Cell<f32>,
}

impl ProgressState {
    pub(crate) fn new(value: f32) -> Self {
        Self {
            value,
            target: Cell::new(value),
        }
    }

    pub(crate) fn target(&self) -> f32 {
        self.target.get()
    }

    pub(crate) fn set_target(&self, value: f32) {
        self.target.set(value);
    }
}
