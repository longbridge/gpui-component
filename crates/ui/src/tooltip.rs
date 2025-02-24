use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, AnyView, App, AppContext, Context, IntoElement,
    ParentElement, Render, Styled, Window,
};

use crate::{text::Text, ActiveTheme};

pub struct Tooltip {
    text: Text,
    element_builder: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
}

impl Tooltip {
    pub fn new(text: impl Into<Text>, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| Self {
            text: text.into(),
            element_builder: None,
        })
        .into()
    }

    pub fn new_element<E, F>(_: &mut Window, cx: &mut App, builder: F) -> AnyView
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        cx.new(|_| Self {
            text: "".into(),
            element_builder: Some(Box::new(move |window, cx| {
                builder(window, cx).into_any_element()
            })),
        })
        .into()
    }
}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            // Wrap in a child, to ensure the left margin is applied to the tooltip
            div()
                .font_family(".SystemUIFont")
                .m_3()
                .bg(cx.theme().popover)
                .text_color(cx.theme().popover_foreground)
                .bg(cx.theme().popover)
                .border_1()
                .border_color(cx.theme().border)
                .shadow_md()
                .rounded(px(6.))
                .py_0p5()
                .px_2()
                .text_sm()
                .map(|this| {
                    if let Some(builder) = &self.element_builder {
                        this.child(builder(window, cx))
                    } else {
                        this.child(self.text.clone())
                    }
                }),
        )
    }
}
