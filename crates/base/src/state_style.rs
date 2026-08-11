use gpui::{StyleRefinement, Styled, prelude::FluentBuilder};

/// A semantic-state style builder.
///
/// Visual modifiers come from GPUI's [`Styled`] trait. Unlike a bare
/// [`StyleRefinement`], this wrapper also supports [`FluentBuilder`] helpers
/// such as `when`, `when_some`, and `when_none`.
#[derive(Default)]
pub struct StateStyle {
    refinement: StyleRefinement,
}

impl StateStyle {
    pub(crate) fn into_refinement(self) -> StyleRefinement {
        self.refinement
    }
}

impl Styled for StateStyle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.refinement
    }
}

impl FluentBuilder for StateStyle {}
