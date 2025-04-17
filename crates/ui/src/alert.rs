use gpui::{
    div, prelude::FluentBuilder as _, px, relative, App, Div, Hsla, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled, Window,
};

use crate::{h_flex, text::Text, ActiveTheme as _, Icon, IconName, Sizable, Size, StyledExt};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AlertVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl AlertVariant {
    fn fg(&self, cx: &App) -> Hsla {
        match self {
            AlertVariant::Info => cx.theme().info,
            AlertVariant::Success => cx.theme().success,
            AlertVariant::Warning => cx.theme().warning,
            AlertVariant::Error => cx.theme().danger,
        }
    }

    fn color(&self, cx: &App) -> Hsla {
        match self {
            AlertVariant::Info => cx.theme().info,
            AlertVariant::Success => cx.theme().success,
            AlertVariant::Warning => cx.theme().warning,
            AlertVariant::Error => cx.theme().danger,
        }
    }
}

/// Alert used to display a message to the user.
#[derive(IntoElement)]
pub struct Alert {
    base: Div,
    variant: AlertVariant,
    icon: Option<Icon>,
    title: Option<SharedString>,
    message: Text,
    size: Size,
}

impl Alert {
    /// Create a new alert with the given message.
    fn new(message: impl Into<Text>) -> Self {
        Self {
            base: div(),
            variant: AlertVariant::default(),
            icon: None,
            title: None,
            message: message.into(),
            size: Size::default(),
        }
    }

    /// Create a new info [`AlertVariant::Info`] with the given message.
    pub fn info(message: impl Into<Text>) -> Self {
        Self::new(message)
            .with_variant(AlertVariant::Info)
            .icon(IconName::Info)
    }

    /// Create a new [`AlertVariant::Success`] alert with the given message.
    pub fn success(message: impl Into<Text>) -> Self {
        Self::new(message)
            .with_variant(AlertVariant::Success)
            .icon(IconName::CircleCheck)
    }

    /// Create a new [`AlertVariant::Warning`] alert with the given message.
    pub fn warning(message: impl Into<Text>) -> Self {
        Self::new(message)
            .with_variant(AlertVariant::Warning)
            .icon(IconName::TriangleAlert)
    }

    /// Create a new [`AlertVariant::Error`] alert with the given message.
    pub fn error(message: impl Into<Text>) -> Self {
        Self::new(message)
            .with_variant(AlertVariant::Error)
            .icon(IconName::CircleX)
    }

    /// Sets the [`AlertVariant`] of the alert.
    pub fn with_variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the icon for the alert.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the title for the alert.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl Sizable for Alert {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Alert {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Alert {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (radius, padding_x, padding_y, gap, line_height, icon_mt) = match self.size {
            Size::XSmall => (cx.theme().radius, px(12.), px(6.), px(6.), 1.2, px(2.5)),
            Size::Small => (cx.theme().radius, px(12.), px(8.), px(6.), 1.2, px(1.5)),
            Size::Large => (
                cx.theme().radius * 3.,
                px(20.),
                px(16.),
                px(12.),
                1.4,
                px(0.),
            ),
            _ => (
                cx.theme().radius * 2.,
                px(16.),
                px(12.),
                px(8.),
                1.3,
                px(1.),
            ),
        };

        let color = self.variant.color(cx);

        self.base.flex_1().child(
            h_flex()
                .w_full()
                .rounded(radius)
                .border_1()
                .border_color(color)
                .bg(color.opacity(0.1))
                .text_color(self.variant.fg(cx))
                .px(padding_x)
                .py(padding_y)
                .gap(gap)
                .overflow_hidden()
                .items_start()
                .map(|this| match self.size {
                    Size::Large => this.text_base(),
                    _ => this.text_sm(),
                })
                .line_height(relative(line_height))
                .child(
                    div().mt(icon_mt).child(
                        self.icon
                            .unwrap_or(IconName::Info.into())
                            .with_size(self.size)
                            .flex_shrink_0(),
                    ),
                )
                .child(
                    div()
                        .overflow_hidden()
                        .when_some(self.title, |this, title| {
                            this.child(
                                div()
                                    .w_full()
                                    .truncate()
                                    .mb_1()
                                    .font_semibold()
                                    .child(title),
                            )
                        })
                        .child(div().overflow_hidden().child(self.message)),
                ),
        )
    }
}
