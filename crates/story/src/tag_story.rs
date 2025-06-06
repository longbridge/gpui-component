use gpui::{
    px, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window,
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
                section("Tag (default)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().child("Tag"))
                        .child(Tag::secondary().child("Secondary"))
                        .child(Tag::outline().child("Outline"))
                        .child(Tag::danger().child("danger"))
                        .child(
                            Tag::custom(yellow_500(), yellow_800(), yellow_500()).child("Custom"),
                        ),
                ),
            )
            .child(
                section("Tag (small)").child(
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
                ),
            )
            .child(
                section("Tag (rounded full)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().rounded_full().child("Tag"))
                        .child(Tag::secondary().rounded_full().child("Secondary"))
                        .child(Tag::outline().rounded_full().child("Outline"))
                        .child(Tag::danger().rounded_full().child("danger"))
                        .child(
                            Tag::custom(yellow_500(), yellow_800(), yellow_500())
                                .rounded_full()
                                .child("Custom"),
                        ),
                ),
            )
            .child(
                section("Tag (small with rounded full)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().small().rounded_full().child("Tag"))
                        .child(Tag::secondary().small().rounded_full().child("Secondary"))
                        .child(Tag::outline().small().rounded_full().child("Outline"))
                        .child(Tag::danger().small().rounded_full().child("danger"))
                        .child(
                            Tag::custom(yellow_500(), yellow_800(), yellow_500())
                                .small()
                                .rounded_full()
                                .child("Custom"),
                        ),
                ),
            )
            .child(
                section("Tag (rounded 0px)").child(
                    h_flex()
                        .gap_2()
                        .child(Tag::primary().small().rounded(px(0.)).child("Tag"))
                        .child(Tag::secondary().small().rounded(px(0.)).child("Secondary"))
                        .child(Tag::outline().small().rounded(px(0.)).child("Outline"))
                        .child(Tag::danger().small().rounded(px(0.)).child("danger"))
                        .child(
                            Tag::custom(yellow_500(), yellow_800(), yellow_500())
                                .small()
                                .rounded(px(0.))
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
