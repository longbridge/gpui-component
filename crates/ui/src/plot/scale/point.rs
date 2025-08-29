// @reference: https://d3js.org/d3-scale/point

use itertools::Itertools;
use num_traits::Zero;

use super::Scale;

#[derive(Clone)]
pub struct ScalePoint<T> {
    domain: Vec<T>,
    range_tick: f32,
}

impl<T> ScalePoint<T>
where
    T: PartialEq,
{
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let len = domain.len();
        let range_tick = if len.is_zero() {
            0.
        } else {
            let range_diff = range
                .iter()
                .minmax()
                .into_option()
                .map_or(0., |(min, max)| max - min);

            if len == 1 {
                range_diff
            } else {
                range_diff / len.saturating_sub(1) as f32
            }
        };

        Self { domain, range_tick }
    }
}

impl<T> Scale<T> for ScalePoint<T>
where
    T: PartialEq,
{
    fn tick(&self, value: &T) -> Option<f32> {
        if self.domain.len() == 1 {
            Some(self.range_tick / 2.)
        } else {
            let index = self.domain.iter().position(|v| v == value)?;
            Some(index as f32 * self.range_tick)
        }
    }

    fn least_index(&self, tick: f32) -> usize {
        if self.domain.is_empty() {
            return 0;
        }

        let index = (tick / self.range_tick).round() as usize;
        index.min(self.domain.len().saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_point() {
        let scale = ScalePoint::new(vec![1, 2, 3], vec![0., 100.]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(50.));
        assert_eq!(scale.tick(&3), Some(100.));
    }

    #[test]
    fn test_scale_point_empty() {
        let scale = ScalePoint::new(vec![], vec![0., 100.]);
        assert_eq!(scale.tick(&1), None);
        assert_eq!(scale.tick(&2), None);
        assert_eq!(scale.tick(&3), None);

        let scale = ScalePoint::new(vec![1, 2, 3], vec![]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(0.));
        assert_eq!(scale.tick(&3), Some(0.));
    }

    #[test]
    fn test_scale_point_single() {
        let scale = ScalePoint::new(vec![1], vec![0., 100.]);
        assert_eq!(scale.tick(&1), Some(50.));
    }
}
