use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, AnyView, App, AppContext, Context, IntoElement,
    ParentElement, Render, Styled, Window,
};

use crate::{h_flex, text::Text, ActiveTheme, Kbd};

enum TooltipContext {
    Text(Text),
    Element(Box<dyn Fn(&mut Window, &mut App) -> AnyElement>),
}

pub struct Tooltip {
    content: TooltipContext,
    key_binding: Option<Kbd>,
}

impl Tooltip {
    /// Create a Tooltip with a text content.
    pub fn new(text: impl Into<Text>) -> Self {
        Self {
            content: TooltipContext::Text(text.into()),
            key_binding: None,
        }
    }

    /// Create a Tooltip with a custom element.
    pub fn element<E, F>(builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        Self {
            key_binding: None,
            content: TooltipContext::Element(Box::new(move |window, cx| {
                builder(window, cx).into_any_element()
            })),
        }
    }

    /// Set KeyBinding information for the tooltip.
    pub(crate) fn key_binding(mut self, kbd: Option<impl Into<Kbd>>) -> Self {
        self.key_binding = kbd.map(Into::into);
        self
    }

    /// Build the tooltip and return it as an `AnyView`.
    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }
}

impl FluentBuilder for Tooltip {}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            // Wrap in a child, to ensure the left margin is applied to the tooltip
            h_flex()
                .font_family(".SystemUIFont")
                .m_3()
                .bg(cx.theme().popover)
                .text_color(cx.theme().popover_foreground)
                .bg(cx.theme().popover)
                .border_1()
                .border_color(cx.theme().border)
                .shadow_md()
                .rounded(px(6.))
                .justify_between()
                .py_0p5()
                .px_2()
                .text_sm()
                .gap_3()
                .map(|this| {
                    this.child(div().map(|this| match self.content {
                        TooltipContext::Text(ref text) => this.child(text.clone()),
                        TooltipContext::Element(ref builder) => this.child(builder(window, cx)),
                    }))
                })
                .when_some(self.key_binding.clone(), |this, kbd| {
                    this.child(
                        div()
                            .text_xs()
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground)
                            .child(kbd.appearance(false)),
                    )
                }),
        )
    }
}
