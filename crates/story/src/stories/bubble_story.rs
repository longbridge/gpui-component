use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    bubble::{Bubble, BubbleReactionSide, BubbleReactions, BubbleVariant},
    button::{Button, ButtonVariants as _},
    h_flex,
    message::MessageAlignment,
    v_flex,
};

use crate::{Story, section};

pub struct BubbleStory {
    focus_handle: FocusHandle,
}

impl BubbleStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for BubbleStory {
    fn title() -> &'static str {
        "Bubble"
    }

    fn description() -> &'static str {
        "A styleable chat surface for text, rich content, and reactions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for BubbleStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BubbleStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Variants")
                    .description("Filled, outline, and ghost cover the shared surface treatments.")
                    .w(px(680.))
                    .v_flex()
                    .gap_4()
                    .child(Bubble::new().child("Filled bubble"))
                    .child(
                        Bubble::new()
                            .with_variant(BubbleVariant::Outline)
                            .child("Outline bubble"),
                    )
                    .child(
                        Bubble::new()
                            .with_variant(BubbleVariant::Ghost)
                            .child("Ghost bubble for rich or unframed content"),
                    ),
            )
            .child(
                section("Alignment")
                    .description("Use the same alignment value as Message.")
                    .w(px(680.))
                    .v_flex()
                    .gap_3()
                    .child(
                        Bubble::new()
                            .bg(cx.theme().secondary)
                            .text_color(cx.theme().secondary_foreground)
                            .child("Incoming message"),
                    )
                    .child(
                        Bubble::new()
                            .alignment(MessageAlignment::End)
                            .child("Outgoing message"),
                    ),
            )
            .child(
                section("Reactions")
                    .description("Reaction controls keep Button semantics and remain replaceable.")
                    .w(px(680.))
                    .py_6()
                    .child(
                        Bubble::new()
                            .with_variant(BubbleVariant::Outline)
                            .child("This bubble has reaction feedback.")
                            .child(
                                BubbleReactions::new().child(
                                    Button::new("bubble-like").ghost().xsmall().label("👍 2"),
                                ),
                            ),
                    )
                    .child(
                        Bubble::new()
                            .alignment(MessageAlignment::End)
                            .child("Reactions can attach to any edge.")
                            .child(
                                BubbleReactions::new()
                                    .side(BubbleReactionSide::Top)
                                    .alignment(MessageAlignment::Start)
                                    .child("✨ 1"),
                            ),
                    ),
            )
            .child(
                section("Rich content")
                    .description("Any GPUI element can be placed directly in the surface.")
                    .w(px(680.))
                    .child(
                        Bubble::new().child(
                            h_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .size_10()
                                        .rounded(cx.theme().radius)
                                        .bg(cx.theme().primary_foreground.opacity(0.18)),
                                )
                                .child(
                                    v_flex()
                                        .child("design-notes.pdf")
                                        .child(div().text_xs().opacity(0.75).child("2.4 MB · PDF")),
                                ),
                        ),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Caller refinements override the surface defaults.")
                    .w(px(680.))
                    .child(
                        Bubble::new()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().green.opacity(0.15))
                            .text_color(cx.theme().green)
                            .border_color(cx.theme().green.opacity(0.35))
                            .child("Custom semantic color"),
                    ),
            )
    }
}
