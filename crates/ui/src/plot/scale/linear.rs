// @reference: https://d3js.org/d3-scale/linear

use itertools::Itertools;
use num_traits::{Num, ToPrimitive};

use super::{sealed::Sealed, Scale};

#[derive(Clone)]
pub struct ScaleLinear<T> {
    domain_len: usize,
    domain_min: T,
    domain_diff: T,
    range_min: f64,
    range_diff: f64,
}

impl<T> ScaleLinear<T>
where
    T: Copy + PartialOrd + Num + ToPrimitive + Sealed,
{
    pub fn new(domain: Vec<T>, range: Vec<f64>) -> Self {
        let (domain_min, domain_max) = domain
            .iter()
            .minmax()
            .into_option()
            .map_or((T::zero(), T::zero()), |(min, max)| (*min, *max));

        let (range_min, range_max) = range
            .iter()
            .minmax()
            .into_option()
            .map_or((0., 0.), |(min, max)| (*min, *max));

        Self {
            domain_len: domain.len(),
            domain_min,
            domain_diff: domain_max - domain_min,
            range_min,
            range_diff: range_max - range_min,
        }
    }
}

impl<T> Scale<T> for ScaleLinear<T>
where
    T: Copy + PartialOrd + Num + ToPrimitive + Sealed,
{
    fn tick(&self, value: &T) -> Option<f64> {
        if self.domain_diff.is_zero() {
            return None;
        }

        let ratio = ((*value - self.domain_min) / self.domain_diff).to_f64()?;

        Some((1. - ratio) * self.range_diff + self.range_min)
    }

    fn least_index(&self, tick: f64) -> usize {
        let index = (tick / self.range_diff).round() as usize;
        index.min(self.domain_len.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_linear_1() {
        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![0., 100.]);
        assert_eq!(scale.tick(&1.), Some(100.));
        assert_eq!(scale.tick(&2.), Some(50.));
        assert_eq!(scale.tick(&3.), Some(0.));
    }

    #[test]
    fn test_scale_linear_2() {
        let scale = ScaleLinear::new(vec![], vec![0., 100.]);
        assert_eq!(scale.tick(&1.), None);
        assert_eq!(scale.tick(&2.), None);
        assert_eq!(scale.tick(&3.), None);

        let scale = ScaleLinear::new(vec![1., 2., 3.], vec![]);
        assert_eq!(scale.tick(&1.), Some(0.));
        assert_eq!(scale.tick(&2.), Some(0.));
        assert_eq!(scale.tick(&3.), Some(0.));
    }
}
