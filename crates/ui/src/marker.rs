use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{ActiveTheme as _, StyledExt as _, h_flex};

/// The visual treatment used by a [`Marker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerVariant {
    /// An inline marker with no additional divider.
    #[default]
    Plain,
    /// A centered marker with semantic divider lines on both sides.
    Separator,
    /// A marker with a semantic bottom border.
    Border,
}

/// A compact, composable row for conversation status and system markers.
///
/// `Marker` intentionally accepts arbitrary children. An icon, text, spinner,
/// or action can be composed directly without introducing fixed icon and
/// content slots. Use [`Styled`] methods on the marker to refine its layout or
/// typography for an application-specific use.
#[derive(IntoElement)]
pub struct Marker {
    style: StyleRefinement,
    separator_style: StyleRefinement,
    variant: MarkerVariant,
    children: Vec<AnyElement>,
}

impl Marker {
    /// Create a plain marker.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            separator_style: StyleRefinement::default(),
            variant: MarkerVariant::default(),
            children: Vec::new(),
        }
    }

    /// Set the visual treatment of the marker.
    pub fn with_variant(mut self, variant: MarkerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Refine the decorative lines used by [`MarkerVariant::Separator`].
    pub fn separator_style(mut self, style: StyleRefinement) -> Self {
        self.separator_style = style;
        self
    }
}

impl Default for Marker {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Marker {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Marker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Marker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let variant = self.variant;
        let separator_style = self.separator_style;

        h_flex()
            .w_full()
            .min_h(tokens.spacing.lg)
            .gap(tokens.spacing.sm)
            .text_size(tokens.typography.sm.size)
            .line_height(tokens.typography.sm.line_height)
            .text_color(tokens.colors.muted_foreground)
            .text_left()
            .when(variant == MarkerVariant::Separator, |this| {
                this.justify_center()
            })
            .when(variant == MarkerVariant::Border, |this| {
                this.border_b_1()
                    .border_color(tokens.colors.border)
                    .pb(tokens.spacing.sm)
            })
            .when(variant == MarkerVariant::Separator, |this| {
                this.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(tokens.spacing.xxs / 2.)
                        .bg(tokens.colors.border)
                        .refine_style(&separator_style),
                )
            })
            .children(self.children)
            .when(variant == MarkerVariant::Separator, |this| {
                this.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(tokens.spacing.xxs / 2.)
                        .bg(tokens.colors.border)
                        .refine_style(&separator_style),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_builder() {
        let marker = Marker::new()
            .with_variant(MarkerVariant::Separator)
            .separator_style(StyleRefinement::default())
            .child("Today");

        assert_eq!(marker.variant, MarkerVariant::Separator);
        assert_eq!(marker.children.len(), 1);
        assert_eq!(Marker::default().variant, MarkerVariant::Plain);
    }
}
