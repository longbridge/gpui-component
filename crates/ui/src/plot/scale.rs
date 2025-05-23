mod band;
mod linear;
mod point;
mod sealed;

pub use band::ScaleBand;
pub use linear::ScaleLinear;
pub use point::ScalePoint;

pub trait Scale<T> {
    fn tick(&self, value: &T) -> Option<f64>;

    fn least_index(&self, tick: f64) -> usize;
}
