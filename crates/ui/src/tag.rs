use crate::{theme::ActiveTheme as _, Sizable, Size};
use gpui::{
    div, prelude::FluentBuilder as _, relative, transparent_black, AnyElement, App, AppContext,
    Div, Hsla, InteractiveElement as _, IntoElement, ParentElement, RenderOnce, Styled, Window,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Danger,
    Custom {
        color: Hsla,
        foreground: Hsla,
        border: Hsla,
    },
}
impl TagVariant {
    fn bg(&self, cx: &App) -> Hsla {
        match self {
            Self::Primary => cx.theme().primary,
            Self::Secondary => cx.theme().secondary,
            Self::Outline => transparent_black(),
            Self::Danger => cx.theme().danger,
            Self::Custom { color, .. } => *color,
        }
    }

    fn border(&self, cx: &App) -> Hsla {
        match self {
            Self::Primary => cx.theme().primary,
            Self::Secondary => cx.theme().secondary,
            Self::Outline => cx.theme().border,
            Self::Danger => cx.theme().danger,
            Self::Custom { border, .. } => *border,
        }
    }

    fn fg(&self, cx: &App) -> Hsla {
        match self {
            Self::Primary => cx.theme().primary_foreground,
            Self::Secondary => cx.theme().secondary_foreground,
            Self::Outline => cx.theme().foreground,
            Self::Danger => cx.theme().danger_foreground,
            Self::Custom { foreground, .. } => *foreground,
        }
    }
}

/// Tag is a small status indicator for UI elements.
///
/// Only support: Medium, Small
#[derive(IntoElement)]
pub struct Tag {
    base: Div,
    variant: TagVariant,
    size: Size,
}
impl Tag {
    fn new() -> Self {
        Self {
            base: div().flex().items_center().rounded_md().border_1(),
            variant: TagVariant::default(),
            size: Size::Medium,
        }
    }

    pub fn with_variant(mut self, variant: TagVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn primary() -> Self {
        Self::new().with_variant(TagVariant::Primary)
    }

    pub fn secondary() -> Self {
        Self::new().with_variant(TagVariant::Secondary)
    }

    pub fn outline() -> Self {
        Self::new().with_variant(TagVariant::Outline)
    }

    pub fn danger() -> Self {
        Self::new().with_variant(TagVariant::Danger)
    }

    pub fn custom(color: Hsla, foreground: Hsla, border: Hsla) -> Self {
        Self::new().with_variant(TagVariant::Custom {
            color,
            foreground,
            border,
        })
    }
}
impl Sizable for Tag {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl ParentElement for Tag {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}
impl RenderOnce for Tag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base
            .line_height(relative(1.3))
            .map(|this| match self.size {
                Size::XSmall | Size::Small => this.text_xs().px_1p5().py_0(),
                _ => this.text_xs().px_2p5().py_0p5(),
            })
            .bg(self.variant.bg(cx))
            .text_color(self.variant.fg(cx))
            .border_color(self.variant.border(cx))
            .hover(|this| this.opacity(0.9))
    }
}
