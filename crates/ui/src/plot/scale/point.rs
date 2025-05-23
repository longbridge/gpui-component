// @reference: https://d3js.org/d3-scale/point

use itertools::Itertools;
use num_traits::Zero;

use super::Scale;

#[derive(Clone)]
pub struct ScalePoint<T> {
    domain: Vec<T>,
    range_tick: f64,
}

impl<T> ScalePoint<T>
where
    T: PartialEq,
{
    pub fn new(domain: Vec<T>, range: Vec<f64>) -> Self {
        let len = domain.len();
        let range_tick = if len.is_zero() {
            0.
        } else {
            let range_diff = range
                .iter()
                .minmax()
                .into_option()
                .map_or(0., |(min, max)| max - min);

            range_diff / (len - 1) as f64
        };

        Self { domain, range_tick }
    }
}

impl<T> Scale<T> for ScalePoint<T>
where
    T: PartialEq,
{
    fn tick(&self, value: &T) -> Option<f64> {
        let index = self.domain.iter().position(|v| v == value)?;
        Some(index as f64 * self.range_tick)
    }

    fn least_index(&self, tick: f64) -> usize {
        let index = (tick / self.range_tick).round() as usize;
        index.min(self.domain.len().saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_point_1() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![0., 100.]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(50.));
        assert_eq!(scale.tick(&3), Some(100.));
    }

    #[test]
    fn test_scale_point_2() {
        let scale = ScalePoint::new(vec![], vec![0., 100.]);
        assert_eq!(scale.tick(&1), None);
        assert_eq!(scale.tick(&2), None);
        assert_eq!(scale.tick(&3), None);

        let scale = ScalePoint::new(vec![1, 2, 3], vec![]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(0.));
        assert_eq!(scale.tick(&3), Some(0.));
    }
}
