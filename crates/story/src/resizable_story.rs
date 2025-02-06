use gpui::{
    div, px, AnyElement, App, AppContext, Context, Entity, Focusable, IntoElement,
    ParentElement as _, Render, SharedString, Styled, Window,
};
use gpui_component::{
    resizable::{h_resizable, resizable_panel, v_resizable, ResizablePanelGroup},
    v_flex, ActiveTheme,
};

pub struct ResizableStory {
    focus_handle: gpui::FocusHandle,
    group1: Entity<ResizablePanelGroup>,
    group2: Entity<ResizablePanelGroup>,
}

impl super::Story for ResizableStory {
    fn title() -> &'static str {
        "Resizable"
    }

    fn description() -> &'static str {
        "The resizable panels."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for ResizableStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl ResizableStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut App) -> Self {
        fn panel_box(content: impl Into<SharedString>, cx: &App) -> AnyElement {
            div()
                .p_4()
                .border_1()
                .border_color(cx.theme().border)
                .size_full()
                .child(content.into())
                .into_any_element()
        }

        let group1 = cx.new(|cx| {
            v_resizable()
                .group(
                    h_resizable()
                        .size(px(150.))
                        .child(
                            resizable_panel()
                                .size(px(300.))
                                .content(|_, cx| panel_box("Left 1 (Min 120px)", cx)),
                            cx,
                        )
                        .child(
                            resizable_panel()
                                .size(px(400.))
                                .content(|_, cx| panel_box("Center 1", cx)),
                            cx,
                        )
                        .child(
                            resizable_panel()
                                .size(px(300.))
                                .content(|_, cx| panel_box("Right (Grow)", cx)),
                            cx,
                        ),
                    cx,
                )
                .child(
                    resizable_panel()
                        .size(px(150.))
                        .content(|_, cx| panel_box("Center (Grow)", cx)),
                    cx,
                )
                .child(
                    resizable_panel()
                        .size(px(210.))
                        .content(|_, cx| panel_box("Bottom", cx)),
                    cx,
                )
        });

        let group2 = cx.new(|cx| {
            h_resizable()
                .child(
                    resizable_panel()
                        .size(px(300.))
                        .content(|_, cx| panel_box("Left 2", cx)),
                    cx,
                )
                .child(
                    resizable_panel()
                        .size(px(400.))
                        .content(|_, cx| panel_box("Right (Grow)", cx)),
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(self.group1.clone())
            .child(self.group2.clone())
    }
}
