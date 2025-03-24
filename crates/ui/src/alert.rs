use gpui::{
    div, prelude::FluentBuilder as _, px, relative, App, IntoElement, ParentElement as _,
    RenderOnce, SharedString, Styled, Window,
};

use crate::{
    h_flex, text::Text, v_flex, ActiveTheme as _, Icon, IconName, Sizable, Size, StyledExt,
};

/// Alert used to display a message to the user.
#[derive(IntoElement)]
pub struct Alert {
    icon: Option<Icon>,
    title: Option<SharedString>,
    message: Text,
    size: Size,
}

impl Alert {
    /// Create a new alert with the given message.
    pub fn new(message: impl Into<Text>) -> Self {
        Self {
            icon: None,
            title: None,
            message: message.into(),
            size: Size::default(),
        }
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

impl RenderOnce for Alert {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (radius, padding_x, padding_y, gap) = match self.size {
            Size::XSmall | Size::Small => (cx.theme().radius, px(12.), px(8.), px(4.)),
            Size::Large => (cx.theme().radius * 3., px(20.), px(16.), px(8.)),
            _ => (cx.theme().radius * 2., px(16.), px(12.), px(6.)),
        };

        h_flex()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().border)
            .px(padding_x)
            .py(padding_y)
            .gap(gap * 2.)
            .overflow_hidden()
            .items_start()
            .child(
                div()
                    .when(self.title.is_none(), |this| this.mt_1())
                    .child(self.icon.unwrap_or(IconName::Info.into()).flex_shrink_0()),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap(gap)
                    .when_some(self.title, |this, title| {
                        this.child(
                            div()
                                .w_full()
                                .truncate()
                                .line_height(relative(1.))
                                .font_semibold()
                                .child(title),
                        )
                    })
                    .child(div().child(self.message)),
            )
    }
}
