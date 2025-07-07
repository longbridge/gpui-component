use gpui::{
    px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::{indicator::Indicator, v_flex, ActiveTheme as _, IconName, Sizable};

use crate::section;

pub struct IndicatorStory {
    focus_handle: gpui::FocusHandle,
    value: f32,
}

impl super::Story for IndicatorStory {
    fn title() -> &'static str {
        "Indicator"
    }

    fn description() -> &'static str {
        "Displays an indicator showing the completion progress of a task."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl IndicatorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            value: 50.,
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value;
    }
}

impl Focusable for IndicatorStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IndicatorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .items_center()
            .gap_y_3()
            .child(section("Indicator").gap_x_2().child(Indicator::new()))
            .child(
                section("Indicator with color")
                    .gap_x_2()
                    .child(Indicator::new().color(cx.theme().blue))
                    .child(Indicator::new().color(cx.theme().green)),
            )
            .child(
                section("Indicator with size")
                    .gap_x_2()
                    .child(Indicator::new().with_size(px(64.)))
                    .child(Indicator::new().large())
                    .child(Indicator::new())
                    .child(Indicator::new().small())
                    .child(Indicator::new().xsmall()),
            )
            .child(
                section("Indicator with Icon")
                    .gap_x_2()
                    .child(Indicator::new().icon(IconName::LoaderCircle))
                    .child(
                        Indicator::new()
                            .icon(IconName::LoaderCircle)
                            .large()
                            .color(cx.theme().cyan),
                    ),
            )
    }
}
