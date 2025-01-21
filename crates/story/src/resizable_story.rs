use gpui::{
    div, px, AnyElement, IntoElement, ParentElement as _, Render, SharedString, Styled, View,
    ViewContext, VisualContext, WindowContext,
};
use ui::ActiveTheme;
use ui::{
    resizable::{h_resizable, resizable_panel, v_resizable, ResizablePanelGroup},
    v_flex,
};

pub struct ResizableStory {
    focus_handle: gpui::FocusHandle,
    group1: View<ResizablePanelGroup>,
    group2: View<ResizablePanelGroup>,
}

impl super::Story for ResizableStory {
    fn title() -> &'static str {
        "Resizable"
    }

    fn description() -> &'static str {
        "The resizable panels."
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }
}

impl gpui::FocusableView for ResizableStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl ResizableStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(|cx| Self::new(cx))
    }

    fn new(cx: &mut WindowContext) -> Self {
        fn panel_box(content: impl Into<SharedString>, cx: &WindowContext) -> AnyElement {
            div()
                .p_4()
                .border_1()
                .border_color(cx.theme().border)
                .size_full()
                .child(content.into())
                .into_any_element()
        }

        let group1 = cx.new_view(|cx| {
            v_resizable(cx)
                .group(
                    h_resizable(cx)
                        .size(px(150.))
                        .child(
                            resizable_panel()
                                .size(px(300.))
                                .content(|cx| panel_box("Left 1 (Min 120px)", cx)),
                            cx,
                        )
                        .child(
                            resizable_panel()
                                .size(px(400.))
                                .content(|cx| panel_box("Center 1", cx)),
                            cx,
                        )
                        .child(
                            resizable_panel()
                                .size(px(300.))
                                .content(|cx| panel_box("Right (Grow)", cx)),
                            cx,
                        ),
                    cx,
                )
                .child(
                    resizable_panel()
                        .size(px(150.))
                        .content(|cx| panel_box("Center (Grow)", cx)),
                    cx,
                )
                .child(
                    resizable_panel()
                        .size(px(210.))
                        .content(|cx| panel_box("Bottom", cx)),
                    cx,
                )
        });

        let group2 = cx.new_view(|cx| {
            h_resizable(cx)
                .child(
                    resizable_panel()
                        .size(px(300.))
                        .content(|cx| panel_box("Left 2", cx)),
                    cx,
                )
                .child(
                    resizable_panel()
                        .size(px(400.))
                        .content(|cx| panel_box("Right (Grow)", cx)),
                    cx,
                )
        });
        Self {
            focus_handle: cx.focus_handle(),
            group1,
            group2,
        }
    }
}

impl Render for ResizableStory {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(self.group1.clone())
            .child(self.group2.clone())
    }
}
