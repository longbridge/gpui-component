use gpui::{
    div, px, AnyElement, App, AppContext, Context, Entity, Focusable, IntoElement,
    ParentElement as _, Pixels, Render, SharedString, Styled, Window,
};
use gpui_component::{
    resizable::{h_resizable, resizable_panel, v_resizable, ResizableState},
    v_flex, ActiveTheme,
};

pub struct ResizableStory {
    focus_handle: gpui::FocusHandle,
    state1: Entity<ResizableState>,
    state2: Entity<ResizableState>,
    state3: Entity<ResizableState>,
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
        let state1 = ResizableState::new(cx);
        let state2 = ResizableState::new(cx);
        let state3 = ResizableState::new(cx);

        Self {
            focus_handle: cx.focus_handle(),
            state1,
            state2,
            state3,
        }
    }
}

fn panel_box(content: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .p_4()
        .border_1()
        .border_color(cx.theme().border)
        .size_full()
        .child(content.into())
        .into_any_element()
}

impl Render for ResizableStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .child(
                div().h(px(800.)).child(
                    v_resizable("resizable-1", self.state1.clone())
                        .group(
                            h_resizable("resizable-1.1", self.state2.clone())
                                .size(px(150.))
                                .child(
                                    resizable_panel()
                                        .size(px(150.))
                                        .size_range(px(120.)..px(300.))
                                        .child(panel_box("Left (120px .. 300px)", cx)),
                                )
                                .child(resizable_panel().child(panel_box("Center", cx)))
                                .child(
                                    resizable_panel()
                                        .size(px(300.))
                                        .child(panel_box("Right", cx)),
                                ),
                        )
                        .child(resizable_panel().child(panel_box("Center", cx)))
                        .child(
                            resizable_panel()
                                .size(px(80.))
                                .size_range(px(80.)..Pixels::MAX)
                                .child(panel_box("Bottom (80px .. 150px)", cx)),
                        ),
                ),
            )
            .child(
                h_resizable("resizable-3", self.state3.clone())
                    .child(
                        resizable_panel()
                            .size(px(200.))
                            .size_range(px(200.)..px(400.))
                            .child(panel_box("Left 2", cx)),
                    )
                    .child(resizable_panel().child(panel_box("Right (Grow)", cx))),
            )
    }
}
