use gpui::{
    px, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window,
};

use gpui_component::{h_flex, indigo_50, indigo_500, tag::Tag, v_flex, ColorName, Sizable};

use crate::section;

pub struct TagStory {
    focus_handle: FocusHandle,
}

impl super::Story for TagStory {
    fn title() -> &'static str {
        "Tag"
    }

    fn description() -> &'static str {
        "A short item that can be used to categorize or label content."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TagStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Tag (default)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().child("Tag"))
                        .child(Tag::secondary().child("Secondary"))
                        .child(Tag::danger().child("Danger"))
                        .child(Tag::success().child("Success"))
                        .child(Tag::warning().child("Warning"))
                        .child(Tag::info().child("Info"))
                        .child(
                            Tag::custom(indigo_500(), indigo_50(), indigo_500()).child("Custom"),
                        ),
                ),
            )
            .child(
                section("Tag (outline)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().outline().child("Tag"))
                        .child(Tag::secondary().outline().child("Secondary"))
                        .child(Tag::danger().outline().child("Danger"))
                        .child(Tag::success().outline().child("Success"))
                        .child(Tag::warning().outline().child("Warning"))
                        .child(Tag::info().outline().child("Info"))
                        .child(
                            Tag::custom(indigo_500(), indigo_500(), indigo_500())
                                .outline()
                                .child("Custom"),
                        ),
                ),
            )
            .child(
                section("Tag (small)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().small().child("Tag"))
                        .child(Tag::secondary().small().child("Secondary"))
                        .child(Tag::danger().small().child("Danger"))
                        .child(Tag::success().small().child("Success"))
                        .child(Tag::warning().small().child("Warning"))
                        .child(Tag::info().small().child("Info")),
                ),
            )
            .child(
                section("Tag (rounded full)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().rounded_full().child("Tag"))
                        .child(Tag::secondary().rounded_full().child("Secondary"))
                        .child(Tag::danger().rounded_full().child("Danger"))
                        .child(Tag::success().rounded_full().child("Success"))
                        .child(Tag::warning().rounded_full().child("Warning"))
                        .child(Tag::info().rounded_full().child("Info")),
                ),
            )
            .child(
                section("Tag (small with rounded full)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().small().rounded_full().child("Tag"))
                        .child(Tag::secondary().small().rounded_full().child("Secondary"))
                        .child(Tag::danger().small().rounded_full().child("Danger"))
                        .child(Tag::success().small().rounded_full().child("Success"))
                        .child(Tag::warning().small().rounded_full().child("Warning"))
                        .child(Tag::info().small().rounded_full().child("Info")),
                ),
            )
            .child(
                section("Tag (rounded 0px)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().small().rounded(px(0.)).child("Tag"))
                        .child(Tag::secondary().small().rounded(px(0.)).child("Secondary"))
                        .child(Tag::danger().small().rounded(px(0.)).child("Danger"))
                        .child(Tag::success().small().rounded(px(0.)).child("Success"))
                        .child(Tag::warning().small().rounded(px(0.)).child("Warning"))
                        .child(Tag::info().small().rounded(px(0.)).child("Info")),
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
