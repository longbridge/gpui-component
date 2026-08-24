use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled as _, Window, px,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
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
                    .w(px(680.))
                    .v_flex()
                    .gap_5()
                    .child(
                        Message::new()
                            .avatar_slot(
                                MessageAvatar::new().child(Avatar::new().name("Alice").small()),
                            )
                            .header(MessageHeader::new().child("Alice").child("10:24 AM"))
                            .content(
                                MessageContent::new().child(
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
                            .avatar(Avatar::new().name("You").small())
                            .header(MessageHeader::new().child("You").child("10:25 AM"))
                            .content(
                                MessageContent::new().child(
                                    Bubble::new().child("Sure — I will send notes shortly."),
                                ),
                            )
                            .footer(MessageFooter::new().child("Delivered")),
                    ),
            )
            .child(
                section("Group")
                    .description("Group consecutive messages while keeping each row composable.")
                    .w(px(680.))
                    .child(
                        MessageGroup::new()
                            .child(
                                Message::new()
                                    .avatar(Avatar::new().name("Alice").small())
                                    .header(MessageHeader::new().child("Alice"))
                                    .content(
                                        MessageContent::new().child(
                                            Bubble::new()
                                                .with_variant(BubbleVariant::Secondary)
                                                .child("I attached the draft."),
                                        ),
                                    ),
                            )
                            .child(
                                Message::new().content(
                                    MessageContent::new().child(
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
                    .w(px(680.))
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
    }
}
