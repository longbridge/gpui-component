mod progress;
mod progress_circle;

pub use progress::Progress;
pub use progress_circle::ProgressCircle;

/// Shared state for progress components.
pub(crate) struct ProgressState {
    pub(crate) value: f32,
}
