use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, div, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide, BubbleReactions, BubbleVariant,
    },
    button::{Button, ButtonVariants as _},
    h_flex,
    message::MessageAlignment,
    v_flex,
};

use crate::{Story, section};

pub struct BubbleStory {
    focus_handle: FocusHandle,
}

fn start_bubble() -> Bubble {
    Bubble::new().alignment(MessageAlignment::Start)
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
                    .description("Semantic variants match the Base UI bubble treatments.")
                    .w(px(680.))
                    .v_flex()
                    .gap_4()
                    .child(start_bubble().child("A strong primary bubble."))
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Secondary)
                            .child("The neutral secondary bubble."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Muted)
                            .child("A lower-emphasis muted bubble."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Tinted)
                            .child("A softly tinted primary bubble."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Outline)
                            .child("A bordered bubble for rich content."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Destructive)
                            .child("A failed action with its reason in text."),
                    )
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Ghost)
                            .child("Ghost content is unframed and can use the full row width."),
                    ),
            )
            .child(
                section("Alignment")
                    .description("Use the same alignment value as Message.")
                    .w(px(680.))
                    .v_flex()
                    .gap_3()
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Secondary)
                            .child("Incoming message"),
                    )
                    .child(
                        start_bubble()
                            .alignment(MessageAlignment::End)
                            .child("Outgoing message"),
                    ),
            )
            .child(
                section("Reactions")
                    .description("Reaction controls keep Button semantics and remain replaceable.")
                    .w(px(680.))
                    .v_flex()
                    .gap_8()
                    .py_6()
                    .child(
                        start_bubble()
                            .with_variant(BubbleVariant::Outline)
                            .child("This bubble has reaction feedback.")
                            .reactions(
                                BubbleReactions::new().child(
                                    Button::new("bubble-like").ghost().xsmall().label("👍 2"),
                                ),
                            ),
                    )
                    .child(
                        Bubble::new()
                            .alignment(MessageAlignment::End)
                            .child("Reactions can attach to any edge.")
                            .reactions(
                                BubbleReactions::new()
                                    .side(BubbleReactionSide::Top)
                                    .alignment(MessageAlignment::Start)
                                    .child("✨ 1"),
                            ),
                    ),
            )
            .child(
                section("Group")
                    .description("Group consecutive bubbles from one sender with an 8 px gap.")
                    .w(px(680.))
                    .child(
                        BubbleGroup::new()
                            .w_full()
                            .child(
                                start_bubble()
                                    .with_variant(BubbleVariant::Secondary)
                                    .child("Can you tell me what changed?"),
                            )
                            .child(
                                start_bubble()
                                    .with_variant(BubbleVariant::Secondary)
                                    .child("The registry route was stale."),
                            ),
                    ),
            )
            .child(
                section("Rich content")
                    .description("Any GPUI element can be placed directly in the surface.")
                    .w(px(680.))
                    .child(
                        start_bubble().child(
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
                        start_bubble().content(
                            BubbleContent::new()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().green.opacity(0.15))
                                .text_color(cx.theme().green)
                                .border_color(cx.theme().green.opacity(0.35))
                                .child("Custom semantic color"),
                        ),
                    ),
            )
    }
}
