use gpui::{Hsla, hsla};

/// Colors used to paint the performance HUD.
///
/// Internal and fixed: the palette is not configurable because its contrast is
/// load bearing. See [`FpsStyle::dark`] — the backdrop alpha is chosen so every
/// foreground stays legible over any window background, and an application that
/// could override it could just as easily make the HUD unreadable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FpsStyle {
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

impl Default for FpsStyle {
    fn default() -> Self {
        Self::dark()
    }
}

impl FpsStyle {
    /// Dark HUD, legible on top of any window background.
    ///
    /// The backdrop is nearly opaque on purpose. GPUI cannot read the pixels
    /// underneath an element, so the HUD has no way to adapt its colors to what
    /// it happens to be covering; the only way to stay readable everywhere is
    /// to stop the background from participating in the composite. At this
    /// alpha every foreground below clears 4.5:1 even over pure white, and the
    /// contrast difference between a white and a black window is under 25%.
    /// Dropping to 0.55 puts `bad` at 1.24:1 over white — invisible.
    ///
    /// The trace colors lean bright and saturated so the chart reads like a
    /// vitals monitor against the dark backdrop.
    pub(crate) fn dark() -> Self {
        Self {
            background: hsla(0., 0., 0.04, 0.92),
            foreground: hsla(0., 0., 0.98, 1.),
            muted: hsla(0., 0., 0.62, 1.),
            good: hsla(0.41, 0.95, 0.56, 1.),
            warn: hsla(0.11, 0.95, 0.6, 1.),
            bad: hsla(0.99, 0.9, 0.62, 1.),
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
