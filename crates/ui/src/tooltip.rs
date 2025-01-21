use gpui::{
    div, px, AnyView, IntoElement, ParentElement, Render, SharedString, Styled, ViewContext,
    VisualContext, WindowContext,
};

use crate::ActiveTheme;

pub struct Tooltip {
    text: SharedString,
}

impl Tooltip {
    pub fn new(text: impl Into<SharedString>, cx: &mut WindowContext) -> AnyView {
        cx.new_view(|_| Self { text: text.into() }).into()
    }
}

impl Render for Tooltip {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
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
                .child(self.text.clone()),
        )
    }
}
