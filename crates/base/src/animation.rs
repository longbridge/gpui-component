use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, ElementId, Hsla, IntoElement, Pixels, Point, Styled, point,
    prelude::FluentBuilder, px,
};
use smallvec::SmallVec;

/// A cubic bezier function like CSS `cubic-bezier`.
///
/// Builder:
///
/// https://cubic-bezier.com
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    // Polynomial form of the unit bezier, where p0 = (0, 0) and p3 = (1, 1).
    let (cx, cy) = (3.0 * x1, 3.0 * y1);
    let (bx, by) = (3.0 * (x2 - x1) - cx, 3.0 * (y2 - y1) - cy);
    let (ax, ay) = (1.0 - cx - bx, 1.0 - cy - by);
    let sample_x = move |t: f32| ((ax * t + bx) * t + cx) * t;
    let sample_y = move |t: f32| ((ay * t + by) * t + cy) * t;
    let slope_x = move |t: f32| (3.0 * ax * t + 2.0 * bx) * t + cx;

    // Solve `x(s) = t` for the curve parameter `s`.
    let solve_s = move |t: f32| {
        let mut s = t;
        for _ in 0..8 {
            let error = sample_x(s) - t;
            if error.abs() < 1e-6 {
                return s;
            }
            let slope = slope_x(s);
            if slope.abs() < 1e-6 {
                break;
            }
            s = (s - error / slope).clamp(0.0, 1.0);
        }

        let (mut low, mut high) = (0.0, 1.0);
        let mut s = t;
        for _ in 0..32 {
            let x = sample_x(s);
            if (x - t).abs() < 1e-6 {
                break;
            }
            if x < t {
                low = s;
            } else {
                high = s;
            }
            s = (low + high) / 2.0;
        }
        s
    };

    move |t: f32| {
        let t = t.clamp(0.0, 1.0);
        // `t` is elapsed progress along x, not the curve parameter: solve
        // `x(s) = t` before sampling y, otherwise the curve reads much slower
        // than the same control points do in CSS. GPUI asserts easing deltas
        // stay within [0, 1], so clamp away solver and rounding error.
        sample_y(solve_s(t)).clamp(0.0, 1.0)
    }
}

// ── Easing presets ──────────────────────────────────────────────────────────

/// Cubic ease-out — fast start, slow end. Good for enter animations.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in — slow start, fast end. Good for exit animations.
pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

/// Cubic ease-in-out — slow start and end. Good for position transitions.
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// ── Lerp trait ──────────────────────────────────────────────────────────────

/// Trait for types that support linear interpolation.
pub trait Lerp: Clone {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl Lerp for Pixels {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        let a: f32 = (*self).into();
        let b: f32 = (*target).into();
        px(a + (b - a) * t)
    }
}

impl Lerp for Point<Pixels> {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        point(
            Lerp::lerp(&self.x, &target.x, t),
            Lerp::lerp(&self.y, &target.y, t),
        )
    }
}

impl Lerp for Hsla {
    /// Interpolate each channel linearly. Intended for transitions between
    /// near-grayscale UI colors (e.g. text colors), where hue interpolation is
    /// irrelevant.
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Hsla {
            h: self.h.lerp(&target.h, t),
            s: self.s.lerp(&target.s, t),
            l: self.l.lerp(&target.l, t),
            a: self.a.lerp(&target.a, t),
        }
    }
}

// ── Transition combinator ───────────────────────────────────────────────────

/// A composable transition that applies concrete fade, slide, and size effects
/// to an element.
///
/// This is distinct from [`crate::motion::Transition`], which is a timing
/// policy for a caller-chosen value and never picks a visual property. Prefer
/// `motion` for new code.
///
/// # Example
///
/// ```ignore
/// EffectTransition::new(Duration::from_millis(150))
///     .ease(ease_out_cubic)
///     .slide_y(px(-4.), px(0.))
///     .fade(0.0, 1.0)
///     .apply(element, "enter-anim")
/// ```
#[derive(Clone)]
pub struct EffectTransition {
    pub duration: Duration,
    easing: Rc<dyn Fn(f32) -> f32>,
    effects: SmallVec<[TransitionEffect; 2]>,
}

#[derive(Clone, Copy)]
enum TransitionEffect {
    SlideY(Pixels, Pixels),
    SlideX(Pixels, Pixels),
    Fade(f32, f32),
    Width(Pixels, Pixels),
    Height(Pixels, Pixels),
}

impl EffectTransition {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: Rc::new(ease_out_cubic),
            effects: SmallVec::new(),
        }
    }

    /// Set the easing function.
    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Rc::new(easing);
        self
    }

    /// Animate vertical offset from `from` to `to`.
    pub fn slide_y(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::SlideY(from, to));
        self
    }

    /// Animate horizontal offset from `from` to `to`.
    pub fn slide_x(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::SlideX(from, to));
        self
    }

    /// Animate opacity from `from` to `to`.
    pub fn fade(mut self, from: f32, to: f32) -> Self {
        self.effects.push(TransitionEffect::Fade(from, to));
        self
    }

    /// Animate width from `from` to `to`.
    pub fn width(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::Width(from, to));
        self
    }

    /// Animate height from `from` to `to`.
    pub fn height(mut self, from: Pixels, to: Pixels) -> Self {
        self.effects.push(TransitionEffect::Height(from, to));
        self
    }

    /// Apply this transition to a Styled element, returning an AnimationElement.
    pub fn apply<E: IntoElement + Styled + 'static>(
        self,
        element: E,
        id: impl Into<ElementId>,
    ) -> gpui::AnimationElement<E> {
        let animation = Animation::new(self.duration).with_easing({
            let easing = self.easing.clone();
            move |t| easing(t)
        });
        let effects = self.effects;
        element.with_animation(id, animation, move |el, delta| {
            let mut el = el;
            for effect in &effects {
                match effect {
                    TransitionEffect::SlideY(from, to) => {
                        el = el.top(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::SlideX(from, to) => {
                        el = el.left(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::Fade(from, to) => {
                        el = el.opacity(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::Width(from, to) => {
                        el = el.w(Lerp::lerp(from, to, delta));
                    }
                    TransitionEffect::Height(from, to) => {
                        el = el.h(Lerp::lerp(from, to, delta));
                    }
                }
            }
            el
        })
    }
}

impl FluentBuilder for EffectTransition {}

/// Former name of [`EffectTransition`].
///
/// Renamed because `motion::Transition` and this type were two different
/// concepts sharing one name.
#[deprecated(since = "0.5.2", note = "renamed to `EffectTransition`")]
pub type Transition = EffectTransition;

#[cfg(test)]
mod tests {
    use super::cubic_bezier;

    #[test]
    fn cubic_bezier_matches_css_ease() {
        let ease = cubic_bezier(0.25, 0.1, 0.25, 1.);
        // Reference values sampled from the CSS `ease` curve.
        for (t, expected) in [
            (0.0, 0.0),
            (0.2, 0.295),
            (0.5, 0.802),
            (0.8, 0.976),
            (1.0, 1.0),
        ] {
            assert!(
                (ease(t) - expected).abs() < 1e-3,
                "ease({t}) = {}, expected {expected}",
                ease(t)
            );
        }
    }

    #[test]
    fn cubic_bezier_with_thirds_x_maps_time_identically() {
        // x1 = 1/3, x2 = 2/3 collapse the x solve to the identity, making the
        // output the plain y polynomial; Dialog relies on this to keep the
        // trajectory it was tuned with before `cubic_bezier` solved for x.
        let ease = cubic_bezier(1. / 3., 0.72, 2. / 3., 1.);
        for step in 0..=100 {
            let t = step as f32 / 100.;
            let one_t = 1. - t;
            let expected = 3. * 0.72 * one_t * one_t * t + 3. * one_t * t * t + t * t * t;
            assert!(
                (ease(t) - expected).abs() < 1e-4,
                "ease({t}) = {}, expected {expected}",
                ease(t)
            );
        }
    }

    #[test]
    fn cubic_bezier_stays_within_unit_range() {
        // GPUI panics when an easing delta leaves [0, 1]; sweep the curves
        // used in-repo densely to catch solver overshoot and rounding error.
        for (x1, y1, x2, y2) in [(0.25, 0.1, 0.25, 1.), (0.32, 0.72, 0., 1.)] {
            let ease = cubic_bezier(x1, y1, x2, y2);
            for step in 0..=10_000 {
                let t = step as f32 / 10_000.;
                let y = ease(t);
                assert!((0.0..=1.0).contains(&y), "ease({t}) = {y} out of range");
            }
        }
    }

    #[test]
    fn cubic_bezier_is_monotonic_and_clamped() {
        let ease = cubic_bezier(0.32, 0.72, 0., 1.);
        assert_eq!(ease(-1.), 0.);
        assert_eq!(ease(2.), 1.);

        let mut previous = 0.;
        for step in 0..=100 {
            let current = ease(step as f32 / 100.);
            assert!(current >= previous - 1e-4, "not monotonic at {step}");
            previous = current;
        }
    }
}
