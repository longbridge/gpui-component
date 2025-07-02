use std::sync::LazyLock;

use gpui::{
    div, img, prelude::FluentBuilder, App, Div, Hsla, ImageSource, InteractiveElement,
    Interactivity, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window,
};

use crate::{avatar::AvatarSized as _, ActiveTheme, Icon, IconName, Sizable, Size, StyledExt};

static AVATAR_COLORS: LazyLock<[Hsla; 17]> = LazyLock::new(|| {
    [
        crate::red_500(),
        crate::orange_500(),
        crate::amber_500(),
        crate::yellow_500(),
        crate::lime_500(),
        crate::green_500(),
        crate::emerald_500(),
        crate::teal_500(),
        crate::cyan_500(),
        crate::sky_500(),
        crate::blue_500(),
        crate::indigo_500(),
        crate::violet_500(),
        crate::fuchsia_500(),
        crate::purple_500(),
        crate::pink_500(),
        crate::rose_500(),
    ]
});

enum AvatarContent {
    Image(ImageSource),
    Text {
        short: SharedString,
        #[allow(unused)]
        full: SharedString,
    },
}

impl Default for AvatarContent {
    fn default() -> Self {
        AvatarContent::Text {
            short: SharedString::new(""),
            full: SharedString::new(""),
        }
    }
}

#[derive(IntoElement)]
pub struct Avatar {
    base: Div,
    style: StyleRefinement,
    content: Option<AvatarContent>,
    placeholder: Icon,
    size: Size,
}

impl Avatar {
    pub fn new() -> Self {
        Self {
            base: div(),
            style: StyleRefinement::default(),
            content: None,
            placeholder: Icon::new(IconName::User),
            size: Size::Medium,
        }
    }

    /// Set to use image source for the avatar.
    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.content = Some(AvatarContent::Image(source.into()));
        self
    }

    /// Set to use text for the avatar, if this is set, the image will be hidden.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        let full: SharedString = text.into();
        let short: SharedString = extract_text_initials(&full).into();

        self.content = Some(AvatarContent::Text { full, short });
        self
    }

    /// Set placeholder icon, default: [`IconName::User`]
    pub fn placeholder(mut self, icon: impl Into<Icon>) -> Self {
        self.placeholder = icon.into();
        self
    }
}
impl Sizable for Avatar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Avatar {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Avatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let corner_radii = self.style.corner_radii.clone();

        let mut inner_style = StyleRefinement::default();
        inner_style.corner_radii = corner_radii;

        const BG_OPACITY: f32 = 0.2;

        self.base
            .avatar_size(self.size)
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .rounded_full()
            .overflow_hidden()
            .bg(cx.theme().secondary)
            .text_color(cx.theme().muted_foreground)
            .border_1()
            .border_color(cx.theme().background)
            .when(self.content.is_none(), |this| {
                this.avatar_text_size(self.size).child(self.placeholder)
            })
            .when_some(self.content, |this, content| match content {
                AvatarContent::Image(source) => this.child(
                    img(source)
                        .avatar_size(self.size)
                        .rounded_full()
                        .refine_style(&inner_style),
                ),
                AvatarContent::Text { short, .. } => {
                    let color_ix = gpui::hash(&short) % AVATAR_COLORS.len() as u64;
                    let color = AVATAR_COLORS[color_ix as usize];

                    this.bg(color.opacity(BG_OPACITY))
                        .text_color(color)
                        .child(div().avatar_text_size(self.size).child(short))
                }
            })
            .refine_style(&self.style)
    }
}

fn extract_text_initials(text: &str) -> String {
    let mut result = text
        .split(" ")
        .map(|word| word.chars().next().map(|c| c.to_string()))
        .flatten()
        .take(2)
        .collect::<Vec<String>>()
        .join("");

    if result.len() == 1 {
        result = text.chars().take(2).collect::<String>();
    }

    result.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_text_initials() {
        assert_eq!(extract_text_initials(&"Jason Lee"), "JL".to_string());
        assert_eq!(extract_text_initials(&"Foo Bar Dar"), "FB".to_string());
        assert_eq!(extract_text_initials(&"huacnlee"), "HU".to_string());
    }
}
