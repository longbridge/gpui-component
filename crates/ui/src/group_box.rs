use gpui::{
    div, prelude::FluentBuilder, relative, AnyElement, App, ElementId, InteractiveElement as _,
    IntoElement, ParentElement, RenderOnce,  StyleRefinement, Styled, Window,
};
use smallvec::SmallVec;

use crate::{v_flex, ActiveTheme, StyledExt as _};

/// The variant of the GroupBox.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum GroupBoxVariant {
    #[default]
    Normal,
    Fill,
    Outline,
}

impl GroupBoxVariant {
    /// Create a GroupBoxVariant from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fill" => GroupBoxVariant::Fill,
            "outline" => GroupBoxVariant::Outline,
            _ => GroupBoxVariant::Normal,
        }
    }

    /// Convert the GroupBoxVariant to a string.
    pub fn as_str(&self) -> &str {
        match self {
            GroupBoxVariant::Normal => "normal",
            GroupBoxVariant::Fill => "fill",
            GroupBoxVariant::Outline => "outline",
        }
    }
}

/// GroupBox is a styled container element that with
/// an optional title to groups related content together.
#[derive(IntoElement)]
pub struct GroupBox {
    id: Option<ElementId>,
    variant: GroupBoxVariant,
    style: StyleRefinement,
    title_style: StyleRefinement,
    title: Option<AnyElement>,
    content_style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
}

impl GroupBox {
    /// Create a new GroupBox.
    pub fn new() -> Self {
        Self {
            id: None,
            variant: GroupBoxVariant::default(),
            style: StyleRefinement::default(),
            title_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            title: None,
            children: SmallVec::new(),
        }
    }

    /// Set the variant of the group box.
    pub fn with_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set to use Fill variant.
    pub fn fill(mut self) -> Self {
        self.variant = GroupBoxVariant::Fill;
        self
    }

    /// Set use outline style of the group box.
    ///
    /// If true, the group box will have a border around it, and no background color.
    pub fn outline(mut self) -> Self {
        self.variant = GroupBoxVariant::Outline;
        self
    }

    /// Set the id of the group box, default is None.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the title of the group box, default is None.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Set the style of the title of the group box to override the default style, default is None.
    pub fn title_style(mut self, style: StyleRefinement) -> Self {
        self.title_style = style;
        self
    }

    /// Set the style of the content of the group box to override the default style, default is None.
    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }
}

impl ParentElement for GroupBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for GroupBox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for GroupBox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (bg, border, has_paddings) = match self.variant {
            GroupBoxVariant::Normal => (None, None, false),
            GroupBoxVariant::Fill => (Some(cx.theme().group_box), None, true),
            GroupBoxVariant::Outline => (None, Some(cx.theme().border), true),
        };

        v_flex()
            .id(self.id.unwrap_or("group-box".into()))
            .w_full()
            .when(has_paddings, |this| this.gap_3())
            .when(!has_paddings, |this| this.gap_4())
            .refine_style(&self.style)
            .when_some(self.title, |this, title| {
                this.child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .line_height(relative(1.))
                        .refine_style(&self.title_style)
                        .child(title),
                )
            })
            .child(
                v_flex()
                    .when_some(bg, |this, bg| this.bg(bg))
                    .when_some(border, |this, border| this.border_color(border).border_1())
                    .text_color(cx.theme().group_box_foreground)
                    .when(has_paddings, |this| this.p_4())
                    .gap_4()
                    .rounded(cx.theme().radius)
                    .refine_style(&self.content_style)
                    .children(self.children),
            )
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_group_variant_from_str() {
        use super::GroupBoxVariant;

        assert_eq!(GroupBoxVariant::from_str("normal"), GroupBoxVariant::Normal);
        assert_eq!(GroupBoxVariant::from_str("fill"), GroupBoxVariant::Fill);
        assert_eq!(
            GroupBoxVariant::from_str("outline"),
            GroupBoxVariant::Outline
        );
        assert_eq!(GroupBoxVariant::from_str("other"), GroupBoxVariant::Normal);

        assert_eq!(GroupBoxVariant::from_str("FILL"), GroupBoxVariant::Fill);
        assert_eq!(
            GroupBoxVariant::from_str("OutLine"),
            GroupBoxVariant::Outline
        );

        assert_eq!(GroupBoxVariant::Normal.as_str(), "normal");
        assert_eq!(GroupBoxVariant::Fill.as_str(), "fill");
        assert_eq!(GroupBoxVariant::Outline.as_str(), "outline");
    }
}
