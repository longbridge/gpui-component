use gpui::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Pixels, Render, SharedString, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme,
    resizable::{h_resizable, resizable_panel, v_resizable},
    v_flex,
};

pub struct ResizableStory {
    focus_handle: FocusHandle,
}

impl super::Story for ResizableStory {
    fn title() -> &'static str {
        "Resizable"
    }

    fn description() -> &'static str {
        "The resizable panels."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

fn panel_box(content: impl Into<SharedString>, _: &App) -> AnyElement {
    div()
        .p_4()
        .size_full()
        .child(content.into())
        .into_any_element()
}

impl Render for ResizableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .child(
                div()
                    .h(px(600.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        v_resizable("resizable-1")
                            .on_resize(|state, _, cx| {
                                println!("Resized: {:?}", state.read(cx).sizes());
                            })
                            .child(
                                h_resizable("resizable-1.1")
                                    .size(px(150.))
                                    .child(
                                        resizable_panel()
                                            .size(px(150.))
                                            .size_range(px(120.)..px(300.))
                                            .child(panel_box("Left (120px .. 300px)", cx)),
                                    )
                                    .child(panel_box("Center", cx))
                                    .child(
                                        resizable_panel()
                                            .size(px(300.))
                                            .child(panel_box("Right", cx)),
                                    ),
                            )
                            .child(panel_box("Center", cx))
                            .child(
                                resizable_panel()
                                    .size(px(80.))
                                    .size_range(px(80.)..Pixels::MAX)
                                    .child(panel_box("Bottom (80px .. 150px)", cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .h(px(400.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_resizable("resizable-3")
                            .child(
                                resizable_panel()
                                    .size(px(200.))
                                    .size_range(px(200.)..px(400.))
                                    .child(panel_box("Left 2", cx)),
                            )
                            .child(panel_box("Right (Grow)", cx)),
                    ),
            )
    }
}
