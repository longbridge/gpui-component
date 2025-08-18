use gpui::{
    div, prelude::FluentBuilder, relative, AnyElement, App, IntoElement, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window,
};
use smallvec::SmallVec;

use crate::{v_flex, ActiveTheme, StyledExt as _};

/// GroupBox is a styled container element that with
/// an optional title to groups related content together.
#[derive(IntoElement)]
pub struct GroupBox {
    style: StyleRefinement,
    title_style: StyleRefinement,
    title: Option<AnyElement>,
    content_style: StyleRefinement,
    outline: bool,
    children: SmallVec<[AnyElement; 1]>,
}

impl GroupBox {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            title_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            title: None,
            outline: false,
            children: SmallVec::new(),
        }
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

    /// Set use outline style of the group box, default is false.
    ///
    /// If true, the group box will have a border around it, and no background color.
    pub fn outline(mut self) -> Self {
        self.outline = true;
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
        v_flex()
            .size_full()
            .gap_2()
            .refine_style(&self.style)
            .when_some(self.title, |this, title| {
                this.child(
                    div()
                        .px_4()
                        .text_color(cx.theme().group_box_foreground)
                        .line_height(relative(1.))
                        .refine_style(&self.title_style)
                        .child(title),
                )
            })
            .child(
                v_flex()
                    .when(!self.outline, |this| this.bg(cx.theme().group_box))
                    .when(self.outline, |this| {
                        this.border_color(cx.theme().border).border_1()
                    })
                    .text_color(cx.theme().group_box_foreground)
                    .p_4()
                    .gap_3()
                    .rounded(cx.theme().radius)
                    .refine_style(&self.content_style)
                    .children(self.children),
            )
    }
}
