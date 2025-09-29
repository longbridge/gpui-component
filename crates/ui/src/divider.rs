use crate::{ActiveTheme, StyledExt};
use gpui::{
    div, prelude::FluentBuilder as _, px, App, Axis, Div, Hsla, IntoElement, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window,
};

/// A divider that can be either vertical or horizontal.
#[derive(IntoElement)]
pub struct Divider {
    base: Div,
    style: StyleRefinement,
    label: Option<SharedString>,
    axis: Axis,
    color: Option<Hsla>,
}

impl Divider {
    /// Creates a vertical divider.
    pub fn vertical() -> Self {
        Self {
            base: div().h_full(),
            axis: Axis::Vertical,
            label: None,
            color: None,
            style: StyleRefinement::default(),
        }
    }

    /// Creates a horizontal divider.
    pub fn horizontal() -> Self {
        Self {
            base: div(),
            axis: Axis::Horizontal,
            label: None,
            color: None,
            style: StyleRefinement::default(),
        }
    }

    /// Sets the label for the divider.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the color for the divider line.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }
}

impl Styled for Divider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Divider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .refine_style(&self.style)
            .child(
                div()
                    .absolute()
                    .map(|this| match self.axis {
                        Axis::Vertical => this.w(px(1.)).h_full(),
                        Axis::Horizontal => this.h(px(1.)).w_full(),
                    })
                    .bg(self.color.unwrap_or(cx.theme().border)),
            )
            .when_some(self.label, |this, label| {
                this.child(
                    div()
                        .px_2()
                        .py_1()
                        .mx_auto()
                        .text_xs()
                        .bg(cx.theme().background)
                        .text_color(cx.theme().muted_foreground)
                        .child(label),
                )
            })
    }
}
