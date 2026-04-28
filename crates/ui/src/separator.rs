use gpui::{App, IntoElement, RenderOnce, StyleRefinement, Styled, Window, div, px};

use crate::{ActiveTheme, StyledExt};

/// The orientation of the separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A visual separator that can be horizontal or vertical.
#[derive(IntoElement)]
pub struct Separator {
    orientation: Orientation,
    style: StyleRefinement,
}

impl Separator {
    /// Create a new horizontal separator.
    pub fn horizontal() -> Self {
        Self {
            orientation: Orientation::Horizontal,
            style: StyleRefinement::default(),
        }
    }

    /// Create a new vertical separator.
    pub fn vertical() -> Self {
        Self {
            orientation: Orientation::Vertical,
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for Separator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = cx.theme().border;
        let base = match self.orientation {
            Orientation::Horizontal => div().w_full().h(px(1.)).bg(color),
            Orientation::Vertical => div().h_full().w(px(1.)).bg(color),
        };
        base.refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_separator_horizontal(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let el = Separator::horizontal().into_any_element();
            // Just verify that the element can be created without panic.
            let _ = el;
        });
    }

    #[gpui::test]
    fn test_separator_vertical(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let el = Separator::vertical().into_any_element();
            let _ = el;
        });
    }
}
