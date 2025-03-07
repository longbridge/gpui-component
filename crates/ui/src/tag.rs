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
    color: Option<ColorName>,
}
impl Tag {
    /// Create a new tag with default variant ([`TagVariant::Outline`]) and size ([`Size::Medium`]).
    pub fn new() -> Self {
        Self {
            base: div().flex().items_center().border_1(),
            variant: TagVariant::default(),
            size: Size::default(),
            color: None,
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

    /// Set special color ([`ColorName`]) for the tag.
    pub fn color(mut self, color: impl Into<ColorName>) -> Self {
        self.color = Some(color.into());
        self
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
        let mut bg = self.variant.bg(cx);
        let mut fg = self.variant.fg(cx);
        let mut border = self.variant.border(cx);

        if let Some(color) = self.color {
            if cx.theme().mode.is_dark() {
                bg = color.color(950);
                fg = color.color(400);
                border = color.color(900);
            } else {
                bg = color.color(50);
                fg = color.color(600);
                border = color.color(200);
            }
        }

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
