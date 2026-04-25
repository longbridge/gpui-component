use gpui::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement as _, Pixels, Render, SharedString, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Sizable as _,
    button::Button,
    h_flex,
    resizable::{ResizableState, h_resizable, resizable_panel, v_resizable},
    v_flex,
};

pub struct ResizableStory {
    focus_handle: FocusHandle,
    programmatic_state: Entity<ResizableState>,
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
            programmatic_state: cx.new(|_| ResizableState::default()),
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
            // Programmatic resize: drive panel sizes via
            // `ResizableState::resize_panel(ix, size, window, cx)`. Buttons
            // mutate the shared state; subscribers (none here) would observe
            // a `ResizablePanelEvent::Resized` just like a drag-finish.
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("compact-left")
                                    .small()
                                    .label("Compact left → 100")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.programmatic_state.update(cx, |state, cx| {
                                            state.resize_panel(0, px(100.), window, cx);
                                        });
                                    })),
                            )
                            .child(
                                Button::new("reset-left")
                                    .small()
                                    .label("Reset left → 200")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.programmatic_state.update(cx, |state, cx| {
                                            state.resize_panel(0, px(200.), window, cx);
                                        });
                                    })),
                            )
                            .child(
                                Button::new("compact-right")
                                    .small()
                                    .label("Compact right → 80")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.programmatic_state.update(cx, |state, cx| {
                                            state.resize_panel(2, px(80.), window, cx);
                                        });
                                    })),
                            )
                            .child(
                                Button::new("reset-right")
                                    .small()
                                    .label("Reset right → 200")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.programmatic_state.update(cx, |state, cx| {
                                            state.resize_panel(2, px(200.), window, cx);
                                        });
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .h(px(200.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_resizable("resizable-programmatic")
                                    .with_state(&self.programmatic_state)
                                    .child(
                                        resizable_panel()
                                            .size(px(200.))
                                            .child(panel_box("Left", cx)),
                                    )
                                    .child(panel_box("Center (grow)", cx))
                                    .child(
                                        resizable_panel()
                                            .size(px(200.))
                                            .child(panel_box("Right", cx)),
                                    ),
                            ),
                    ),
            )
    }
}
