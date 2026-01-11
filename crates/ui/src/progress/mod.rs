mod circle;
mod linear;

pub use circle::ProgressCircle;
pub use linear::Progress;

/// Shared state for progress components.
pub(crate) struct ProgressState {
    pub(crate) value: f32,
}
