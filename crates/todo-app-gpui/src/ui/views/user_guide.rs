use gpui::{App, AppContext, Context, Entity, Focusable, ParentElement, Render, Styled, Window};

use gpui_component::{dock::PanelControl, text::TextView, v_flex};

use crate::ui::components::ViewKit;

pub struct UserGuide {
    focus_handle: gpui::FocusHandle,
}

impl UserGuide {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl ViewKit for UserGuide {
    fn title() -> &'static str {
        "使用指南"
    }

    fn description() -> &'static str {
        "一款基于LLM待办事项实用工具的使用指南"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for UserGuide {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for UserGuide {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex().p_4().gap_5().child(TextView::markdown(
            "user_guid",
            include_str!("user_guide.md"),
        ))
    }
}
