use crate::{theme::ActiveTheme as _, ColorName, Sizable, Size};
use gpui::{
    div, prelude::FluentBuilder as _, relative, transparent_black, AnyElement, App, Div, Hsla,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce, Styled, Window,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagVariant {
    #[default]
    Outline,
    Primary,
    Secondary,
    Danger,
    Color(ColorName),
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
            Self::Color(color) => {
                if cx.theme().is_dark() {
                    color.scale(950).opacity(0.5)
                } else {
                    color.scale(50)
                }
            }
            Self::Custom { color, .. } => *color,
        }
    }

    fn border(&self, cx: &App) -> Hsla {
        match self {
            Self::Primary => cx.theme().primary,
            Self::Secondary => cx.theme().secondary,
            Self::Outline => cx.theme().border,
            Self::Danger => cx.theme().danger,
            Self::Color(color) => {
                if cx.theme().is_dark() {
                    color.scale(800).opacity(0.5)
                } else {
                    color.scale(200)
                }
            }
            Self::Custom { border, .. } => *border,
        }
    }

    fn fg(&self, cx: &App) -> Hsla {
        match self {
            Self::Primary => cx.theme().primary_foreground,
            Self::Secondary => cx.theme().secondary_foreground,
            Self::Outline => cx.theme().foreground,
            Self::Danger => cx.theme().danger_foreground,
            Self::Color(color) => {
                if cx.theme().is_dark() {
                    color.scale(300)
                } else {
                    color.scale(600)
                }
            }
            Self::Custom { foreground, .. } => *foreground,
        }
    }
}

/// Tag is a small status indicator.
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
            base: div().flex().items_center().border_1(),
            variant: TagVariant::default(),
            size: Size::default(),
        }
    }

    pub fn with_variant(mut self, variant: TagVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Create a new tag with default variant ([`TagVariant::Primary`]).
    pub fn primary() -> Self {
        Self::new().with_variant(TagVariant::Primary)
    }

    /// Create a new tag with default variant ([`TagVariant::Secondary`]).
    pub fn secondary() -> Self {
        Self::new().with_variant(TagVariant::Secondary)
    }

    /// Create a new tag with default variant ([`TagVariant::Outline`]).
    ///
    /// See also [`Tag::new`].
    pub fn outline() -> Self {
        Self::new().with_variant(TagVariant::Outline)
    }

    /// Create a new tag with default variant ([`TagVariant::Danger`]).
    pub fn danger() -> Self {
        Self::new().with_variant(TagVariant::Danger)
    }

    /// Create a new tag with default variant ([`TagVariant::Custom`]).
    pub fn custom(color: Hsla, foreground: Hsla, border: Hsla) -> Self {
        Self::new().with_variant(TagVariant::Custom {
            color,
            foreground,
            border,
        })
    }

    /// Create a new tag with default variant ([`TagVariant::Color`]).
    pub fn color(color: impl Into<ColorName>) -> Self {
        Self::new().with_variant(TagVariant::Color(color.into()))
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
        let bg = self.variant.bg(cx);
        let fg = self.variant.fg(cx);
        let border = self.variant.border(cx);

        self.base
            .line_height(relative(1.3))
            .text_xs()
            .map(|this| match self.size {
                Size::XSmall | Size::Small => this.px_1p5().py_0().rounded(cx.theme().radius / 2.),
                _ => this.px_2p5().py_0p5().rounded(cx.theme().radius),
            })
            .bg(bg)
            .text_color(fg)
            .border_color(border)
            .hover(|this| this.opacity(0.9))
    }
}
