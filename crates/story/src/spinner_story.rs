use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use gpui_component::{ActiveTheme as _, IconName, Sizable, spinner::Spinner, v_flex};

use crate::section;

pub struct SpinnerStory {
    focus_handle: gpui::FocusHandle,
    value: f32,
}

impl super::Story for SpinnerStory {
    fn title() -> &'static str {
        "Spinner"
    }

    fn description() -> &'static str {
        "Displays an spinner showing the completion progress of a task."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SpinnerStory {
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

impl Focusable for SpinnerStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SpinnerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .items_center()
            .size_full()
            .gap_6()
            .child(section("Spinner").gap_x_2().child(Spinner::new()))
            .child(
                section("Spinner with color")
                    .gap_x_2()
                    .child(Spinner::new().color(cx.theme().blue))
                    .child(Spinner::new().color(cx.theme().green)),
            )
            .child(
                section("Spinner with size")
                    .gap_x_2()
                    .child(Spinner::new().with_size(px(64.)))
                    .child(Spinner::new().large())
                    .child(Spinner::new())
                    .child(Spinner::new().small())
                    .child(Spinner::new().xsmall()),
            )
            .child(
                section("Spinner with Icon")
                    .gap_x_2()
                    .child(Spinner::new().icon(IconName::LoaderCircle))
                    .child(
                        Spinner::new()
                            .icon(IconName::LoaderCircle)
                            .large()
                            .color(cx.theme().cyan),
                    ),
            )
    }
}
