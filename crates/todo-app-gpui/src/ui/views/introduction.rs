use crate::ui::components::ViewKit;
use gpui::*;
use gpui_component::{dock::PanelControl, text::TextView, *};

pub struct Introduction {
    focus_handle: gpui::FocusHandle,
}

impl Introduction {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl ViewKit for Introduction {
    fn title() -> &'static str {
        "简介"
    }

    fn description() -> &'static str {
        "一款基于LLM待办事项实用工具"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for Introduction {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Introduction {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .text_xs()
            .child(TextView::markdown("intro", include_str!("introduction.md")))
    }
}
