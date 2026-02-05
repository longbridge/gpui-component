use crate::{ActiveTheme, StyledExt, v_flex};

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, prelude::FluentBuilder,
};

#[derive(IntoElement)]
pub struct Card {
    id: ElementId,
    children: Vec<AnyElement>,
    title: Option<SharedString>,
    footer: Vec<AnyElement>,
    // QA: style 是什么作用？怎么使用？
    style: StyleRefinement,
}

impl Card {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            // QA: ElementId 什么作用
            id: id.clone(),
            title: None,
            footer: Vec::new(),
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn footer(mut self, elements: impl IntoElement) -> Self {
        self.footer.push(elements.into_any_element());
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .id(self.id.clone())
            .refine_style(&self.style)
            .w_full()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            // header
            .when(self.title.is_some(), |this| {
                this.child(
                    v_flex()
                        .child(self.title.unwrap())
                        .bg(cx.theme().title_bar)
                        .font_bold()
                        .p_4()
                        .border_b_1()
                        .border_color(cx.theme().border),
                )
            })
            // content
            .child(v_flex().relative().p_4().children(self.children))
            // footer
            .when(self.footer.len() > 0, |this| {
                this.child(
                    v_flex()
                        .p_4()
                        .bg(cx.theme().title_bar)
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .children(self.footer),
                )
            })
    }
}
