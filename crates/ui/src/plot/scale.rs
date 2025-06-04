mod band;
mod linear;
mod point;
mod sealed;

pub use band::ScaleBand;
pub use linear::ScaleLinear;
pub use point::ScalePoint;
pub(crate) use sealed::Sealed;

pub trait Scale<T> {
    /// Get the tick of the scale.
    fn tick(&self, value: &T) -> Option<f64>;

    /// Get the least index of the scale.
    fn least_index(&self, tick: f64) -> usize;
}
