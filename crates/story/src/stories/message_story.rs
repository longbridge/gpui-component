use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, StyleRefinement, Styled as _, Window, rems,
};
use gpui_component::{
    ActiveTheme as _, StyledExt as _,
    avatar::Avatar,
    bubble::{Bubble, BubbleVariant},
    message::{
        Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter, MessageGroup,
        MessageHeader,
    },
    v_flex,
};

use crate::{Story, section};

pub struct MessageStory {
    focus_handle: FocusHandle,
}

impl MessageStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Story for MessageStory {
    fn title() -> &'static str {
        "Message"
    }

    fn description() -> &'static str {
        "Compose sender identity, metadata, rich content, and message actions."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for MessageStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MessageStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Alignment")
                    .description("The message owns alignment for all of its named slots.")
                    .w(rems(42.5))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .avatar_slot(
                                MessageAvatar::new().child(Avatar::new().name("Alice").size_8()),
                            )
                            .header(MessageHeader::new().child("Alice").child("10:24 AM"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Secondary)
                                        .child("Can you review this?"),
                                ),
                            )
                            .footer(MessageFooter::new().child("Read")),
                    )
                    .child(
                        Message::new()
                            .alignment(MessageAlignment::End)
                            .avatar(Avatar::new().name("You").size_8())
                            .header(MessageHeader::new().child("You").child("10:25 AM"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new().child("Sure — I will send notes shortly."),
                                ),
                            )
                            .footer(MessageFooter::new().child("Delivered")),
                    ),
            )
            .child(
                section("Group")
                    .description("Group consecutive messages while keeping each row composable.")
                    .w(rems(42.5))
                    .child(
                        MessageGroup::new()
                            .child(
                                Message::new()
                                    .avatar(Avatar::new().name("Alice").size_8())
                                    .header(MessageHeader::new().child("Alice"))
                                    .content(
                                        MessageContent::new().bubble(
                                            Bubble::new()
                                                .with_variant(BubbleVariant::Secondary)
                                                .child("I attached the draft."),
                                        ),
                                    ),
                            )
                            .child(
                                Message::new()
                                    .avatar_slot(MessageAvatar::new().bg(cx.theme().transparent))
                                    .content(
                                        MessageContent::new().bubble(
                                            Bubble::new()
                                                .with_variant(BubbleVariant::Secondary)
                                                .child("The second page needs attention."),
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                section("Custom style")
                    .description("Every structural part accepts GPUI style refinements.")
                    .w(rems(42.5))
                    .child(
                        Message::new()
                            .p_3()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().muted.opacity(0.35))
                            .header(MessageHeader::new().px_0().child("System"))
                            .content(
                                MessageContent::new().child("The conversation has been archived."),
                            )
                            .footer(MessageFooter::new().px_0().child("Just now")),
                    ),
            )
            .child(
                section("Ghost surface")
                    .description("Typed ghost bubbles automatically remove metadata insets.")
                    .w(rems(42.5))
                    .child(
                        Message::new()
                            .with_stack_style(StyleRefinement::default().gap_3())
                            .header(MessageHeader::new().child("System").child("Just now"))
                            .content(
                                MessageContent::new().bubble(
                                    Bubble::new()
                                        .with_variant(BubbleVariant::Ghost)
                                        .child("The conversation has been archived."),
                                ),
                            )
                            .footer(MessageFooter::new().child("No further action required")),
                    ),
            )
    }
}
