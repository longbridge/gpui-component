use gpui::{
    px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::{skeleton::Skeleton, v_flex};

use crate::section;

pub struct SkeletonStory {
    focus_handle: gpui::FocusHandle,
    value: f32,
}

impl super::Story for SkeletonStory {
    fn title() -> &'static str {
        "Skeleton"
    }

    fn description() -> &'static str {
        "Use to show a placeholder while content is loading."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SkeletonStory {
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

impl Focusable for SkeletonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SkeletonStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .items_center()
            .gap_y_3()
            .child(
                section("Skeleton")
                    .max_w_md()
                    .child(Skeleton::new().size_12().rounded_full())
                    .child(
                        v_flex()
                            .gap_2()
                            .child(Skeleton::new().w(px(250.)).h_4().rounded_md())
                            .child(Skeleton::new().w(px(200.)).h_4().rounded_md()),
                    ),
            )
            .child(
                section("Card").max_w_md().child(
                    v_flex()
                        .gap_2()
                        .child(Skeleton::new().w(px(250.)).h(px(125.)).rounded_md())
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Skeleton::new().w(px(250.)).h_4().rounded_md())
                                .child(Skeleton::new().w(px(200.)).h_4().rounded_md()),
                        ),
                ),
            )
    }
}
