use gpui::{Hsla, hsla};

/// Colors used to paint the performance HUD.
///
/// The HUD deliberately does not read from any theme so that it can be dropped
/// into any GPUI application. Applications that do have a theme can map their
/// own tokens onto these fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerfStyle {
    /// Backdrop behind the HUD.
    pub background: Hsla,
    /// Primary readouts (the FPS number).
    pub foreground: Hsla,
    /// Secondary readouts (units, labels, resource row).
    pub muted: Hsla,
    /// Frames that finished within the frame budget.
    pub good: Hsla,
    /// Frames that overran the budget but stayed within twice of it.
    pub warn: Hsla,
    /// Frames that overran twice the budget.
    pub bad: Hsla,
}

impl Default for PerfStyle {
    fn default() -> Self {
        Self::dark()
    }
}

impl PerfStyle {
    /// Translucent dark HUD, legible on top of most content.
    ///
    /// The trace colors lean bright and saturated so the chart reads like a
    /// vitals monitor against the dark backdrop.
    pub fn dark() -> Self {
        Self {
            background: hsla(0., 0., 0.04, 0.82),
            foreground: hsla(0., 0., 0.98, 1.),
            muted: hsla(0., 0., 0.62, 1.),
            good: hsla(0.41, 0.95, 0.56, 1.),
            warn: hsla(0.11, 0.95, 0.6, 1.),
            bad: hsla(0.99, 0.9, 0.62, 1.),
        }
    }

    /// Translucent light HUD, for applications with dark content.
    pub fn light() -> Self {
        Self {
            background: hsla(0., 0., 1., 0.82),
            foreground: hsla(0., 0., 0.1, 1.),
            muted: hsla(0., 0., 0.4, 1.),
            good: hsla(0.38, 0.6, 0.38, 1.),
            warn: hsla(0.09, 0.8, 0.42, 1.),
            bad: hsla(0.01, 0.7, 0.48, 1.),
        }
    }

    /// The color for a frame that took `frame_secs` against `budget_secs`.
    pub(crate) fn level_color(&self, frame_secs: f32, budget_secs: f32) -> Hsla {
        if frame_secs <= budget_secs {
            self.good
        } else if frame_secs <= budget_secs * 2. {
            self.warn
        } else {
            self.bad
        }
    }
}
