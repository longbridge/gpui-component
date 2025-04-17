use gpui::{App, AppContext, Context, Entity, Focusable, ParentElement, Render, Styled, Window};

use gpui_component::{dock::PanelControl, text::TextView, v_flex};

use crate::Story;

pub struct WelcomeStory {
    focus_handle: gpui::FocusHandle,
}

impl WelcomeStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Story for WelcomeStory {
    fn title() -> &'static str {
        "Introduction"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for WelcomeStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WelcomeStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex().p_4().gap_5().child(TextView::markdown(
            "intro",
            include_str!("../../../README.md"),
        ))
    }
}
