use gpui::{App, AppContext, Context, Entity, Focusable, ParentElement, Render, Styled, Window};

use gpui_component::{dock::PanelControl, text::TextView, v_flex};

use crate::ui::components::ViewKit;

pub struct Profile {
    focus_handle: gpui::FocusHandle,
}

impl Profile {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl ViewKit for Profile {
    fn title() -> &'static str {
        "个人资料"
    }

    fn description() -> &'static str {
        "设置您的个人资料和偏好，有助于个性化您的体验"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for Profile {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Profile {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(TextView::markdown("intro", include_str!("introduction.md")))
    }
}
