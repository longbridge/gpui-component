// @reference: https://d3js.org/d3-scale/band

use itertools::Itertools;
use num_traits::Zero;

use super::Scale;

#[derive(Clone)]
pub struct ScaleBand<T> {
    domain: Vec<T>,
    range_diff: f64,
    avg_width: f64,
    padding_inner: f64,
    padding_outer: f64,
}

impl<T> ScaleBand<T> {
    pub fn new(domain: Vec<T>, range: Vec<f64>) -> Self {
        let len = domain.len() as f64;
        let range_diff = range
            .iter()
            .minmax()
            .into_option()
            .map_or(0., |(min, max)| max - min);

        Self {
            domain,
            range_diff,
            avg_width: if len.is_zero() { 0. } else { range_diff / len },
            padding_inner: 0.,
            padding_outer: 0.,
        }
    }

    pub fn band_width(&self) -> f64 {
        (self.avg_width * (1. - self.padding_inner)).min(30.)
    }

    pub fn padding_inner(mut self, padding_inner: f64) -> Self {
        self.padding_inner = padding_inner;
        self
    }

    pub fn padding_outer(mut self, padding_outer: f64) -> Self {
        self.padding_outer = padding_outer;
        self
    }
}

impl<T> Scale<T> for ScaleBand<T>
where
    T: PartialEq,
{
    fn tick(&self, value: &T) -> Option<f64> {
        let index = self.domain.iter().position(|v| v == value)?;
        let domain_len = self.domain.len();

        // When there's only one element, place it in the center.
        if domain_len == 1 {
            return Some((self.range_diff - self.band_width()) / 2.);
        }

        let ratio = 1. + self.padding_inner / (self.domain.len() - 1) as f64;
        let padding_outer_width = self.avg_width * self.padding_outer;
        let avg_width = (self.range_diff - padding_outer_width * 2.) / self.domain.len() as f64;
        Some(index as f64 * avg_width * ratio + padding_outer_width)
    }

    fn least_index(&self, tick: f64) -> usize {
        let index = (tick / self.avg_width).round() as usize;
        index.min(self.domain.len().saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_band() {
        let scale = ScaleBand::new(vec![1, 2, 3], vec![0., 90.]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(30.));
        assert_eq!(scale.tick(&3), Some(60.));
        assert_eq!(scale.band_width(), 30.);
    }

    #[test]
    fn test_scale_band_zero() {
        let scale = ScaleBand::new(vec![], vec![0., 90.]);
        assert_eq!(scale.tick(&1), None);
        assert_eq!(scale.tick(&2), None);
        assert_eq!(scale.tick(&3), None);
        assert_eq!(scale.band_width(), 0.);

        let scale = ScaleBand::new(vec![1, 2, 3], vec![]);
        assert_eq!(scale.tick(&1), Some(0.));
        assert_eq!(scale.tick(&2), Some(0.));
        assert_eq!(scale.tick(&3), Some(0.));
        assert_eq!(scale.band_width(), 0.);
    }
}
