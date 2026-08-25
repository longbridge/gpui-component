use std::rc::Rc;

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StyleRefinement, Styled as _, Window,
    div, prelude::FluentBuilder as _, rems,
};
use gpui_component::{
    ActiveTheme as _, StyledExt as _,
    bubble::{Bubble, BubbleVariant},
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    marker::{Marker, MarkerContent, MarkerVariant},
    message::{Message, MessageAlignment, MessageContent, MessageHeader},
    message_scroller::{MessageScroller, MessageScrollerState},
    v_flex,
};

use crate::{Story, section};

#[derive(Clone)]
struct DemoMessage {
    id: usize,
    author: SharedString,
    body: SharedString,
    sent: bool,
}

impl DemoMessage {
    fn new(id: usize, sent: bool, body: impl Into<SharedString>) -> Self {
        Self {
            id,
            author: if sent { "You".into() } else { "Alice".into() },
            body: body.into(),
            sent,
        }
    }
}

pub struct MessageScrollerStory {
    focus_handle: FocusHandle,
    scroller: Entity<MessageScrollerState>,
    composer: Entity<InputState>,
    messages: Vec<DemoMessage>,
    unread_index: usize,
    next_id: usize,
}

impl MessageScrollerStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let messages = (0..32)
            .map(|index| {
                let sent = index % 3 == 2;
                let body = if index % 5 == 0 {
                    format!(
                        "Message {} has a second line so variable-height rows exercise the virtual list anchor.",
                        index + 1
                    )
                } else {
                    format!("Conversation message {}", index + 1)
                };
                DemoMessage::new(index, sent, body)
            })
            .collect::<Vec<_>>();
        let scroller = cx.new(|cx| MessageScrollerState::new(messages.len(), cx));
        cx.observe(&scroller, |_, _, cx| cx.notify()).detach();
        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("Write a message…"));

        Self {
            focus_handle: cx.focus_handle(),
            scroller,
            composer,
            messages,
            unread_index: 18,
            next_id: 32,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn append_message(&mut self, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(DemoMessage::new(
            id,
            true,
            format!("New message {}", id + 1),
        ));
        self.scroller
            .update(cx, |state, cx| _ = state.append(1, cx));
        cx.notify();
    }

    fn prepend_history(&mut self, cx: &mut Context<Self>) {
        const COUNT: usize = 5;
        let first_id = self.next_id;
        self.next_id += COUNT;
        let earlier = (0..COUNT).map(|offset| {
            DemoMessage::new(
                first_id + offset,
                false,
                format!("Earlier history {}", offset + 1),
            )
        });

        self.messages.splice(0..0, earlier);
        self.unread_index += COUNT;
        self.scroller
            .update(cx, |state, cx| _ = state.prepend(COUNT, cx));
        cx.notify();
    }

    fn send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.composer.read(cx).value();
        if body.trim().is_empty() {
            return;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(DemoMessage::new(id, true, body));
        self.composer
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.scroller
            .update(cx, |state, cx| _ = state.append(1, cx));
        cx.notify();
    }

    fn scroll_to_unread(&mut self, cx: &mut Context<Self>) {
        let unread_index = self.unread_index;
        self.scroller.update(cx, |state, cx| {
            _ = state.scroll_to_unread(unread_index, cx);
        });
    }
}

impl Story for MessageScrollerStory {
    fn title() -> &'static str {
        "MessageScroller"
    }

    fn description() -> &'static str {
        "A virtualized message list with tail following, unread navigation, and anchor preservation."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for MessageScrollerStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MessageScrollerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let messages = Rc::new(self.messages.clone());
        let unread_index = self.unread_index;
        let status = {
            let state = self.scroller.read(cx);
            format!(
                "Following tail: {} · Scrolled up: {}",
                state.is_following_tail(),
                state.is_scrolled_up()
            )
        };

        v_flex().gap_4().child(
            section("Conversation")
                .description(
                    "Scroll upward, append a row, jump to unread, or prepend history to exercise each behavior.",
                )
                .w(rems(45.))
                .v_flex()
                .gap_3()
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("message-scroller-append")
                                .label("Append")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.append_message(cx)
                                })),
                        )
                        .child(
                            Button::new("message-scroller-prepend")
                                .label("Prepend history")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.prepend_history(cx)
                                })),
                        )
                        .child(
                            Button::new("message-scroller-unread")
                                .label("Scroll to unread")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.scroll_to_unread(cx)
                                })),
                        ),
                )
                .child(
                    v_flex()
                        .w_96()
                        .max_w_full()
                        .h(rems(35.))
                        .overflow_hidden()
                        .rounded(cx.theme().radius_4xl())
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().group_box)
                        .text_color(cx.theme().group_box_foreground)
                        .child(
                            v_flex()
                                .w_full()
                                .gap_1()
                                .p_5()
                                .border_b_1()
                                .border_color(cx.theme().border)
                                .child(div().font_semibold().child("New chat"))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("How can I help you today?"),
                                ),
                        )
                        .child(
                            div().flex_1().min_h_0().child(
                                MessageScroller::new(
                                    "message-scroller-demo",
                                    self.scroller.clone(),
                                    move |index, _, _| {
                                        let Some(message) = messages.get(index).cloned() else {
                                            return div().into_any_element();
                                        };
                                        let alignment = if message.sent {
                                            MessageAlignment::End
                                        } else {
                                            MessageAlignment::Start
                                        };
                                        let bubble = Bubble::new()
                                            .when(!message.sent, |bubble| {
                                                bubble.with_variant(BubbleVariant::Secondary)
                                            })
                                            .child(message.body);
                                        let row = div()
                                            .id(("message-scroller-row", message.id))
                                            .w_full()
                                            .child(
                                                Message::new()
                                                    .alignment(alignment)
                                                    .header(
                                                        MessageHeader::new().child(message.author),
                                                    )
                                                    .content(MessageContent::new().bubble(bubble)),
                                            );

                                        v_flex()
                                            .w_full()
                                            .min_w_0()
                                            .gap_3()
                                            .when(index == unread_index, |this| {
                                                this.child(
                                                    Marker::new()
                                                        .with_variant(MarkerVariant::Separator)
                                                        .content(
                                                            MarkerContent::new().child("Unread"),
                                                        ),
                                                )
                                            })
                                            .child(row)
                                            .into_any_element()
                                    },
                                )
                                .with_list_style(StyleRefinement::default().p_5()),
                            ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap_2()
                                .p_5()
                                .border_t_1()
                                .border_color(cx.theme().border)
                                .child(Input::new(&self.composer).flex_1())
                                .child(
                                    Button::new("message-scroller-send")
                                        .primary()
                                        .label("Send")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.send_message(window, cx)
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(status),
                ),
        )
    }
}
