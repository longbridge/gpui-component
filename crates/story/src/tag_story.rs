use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window,
};

use gpui_component::{h_flex, tag::Tag, v_flex, yellow_500, yellow_800, ColorName, Sizable};

use crate::section;

pub struct TagStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for TagStory {
    fn title() -> &'static str {
        "Tag"
    }

    fn description() -> &'static str {
        "A short item that can be used to categorize or label content."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl TagStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}
impl Focusable for TagStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TagStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Tag")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Tag::primary().small().child("Tag"))
                            .child(Tag::secondary().small().child("Secondary"))
                            .child(Tag::outline().small().child("Outline"))
                            .child(Tag::danger().small().child("danger"))
                            .child(
                                Tag::custom(yellow_500(), yellow_800(), yellow_500())
                                    .small()
                                    .child("Custom"),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Tag::primary().child("Tag"))
                            .child(Tag::secondary().child("Secondary"))
                            .child(Tag::outline().child("Outline"))
                            .child(Tag::danger().child("danger"))
                            .child(
                                Tag::custom(yellow_500(), yellow_800(), yellow_500())
                                    .child("Custom"),
                            ),
                    ),
            )
            .child(
                section("Color Tags").child(
                    v_flex().gap_4().child(
                        h_flex().gap_2().flex_wrap().children(
                            ColorName::all()
                                .into_iter()
                                .filter(|color| *color != ColorName::Gray)
                                .map(|color| Tag::color(color).child(color.to_string())),
                        ),
                    ),
                ),
            )
    }
}
