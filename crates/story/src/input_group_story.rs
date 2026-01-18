use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window, div,
};

use crate::section;
use gpui_component::{button::*, input::*, spinner::Spinner, *};

pub fn init(_: &mut App) {}

pub struct InputGroupStory {
    search_input: Entity<InputState>,
    url_input: Entity<InputState>,
    username_input: Entity<InputState>,
    chat_input: Entity<InputState>,
}

impl super::Story for InputGroupStory {
    fn title() -> &'static str {
        "InputGroup"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl InputGroupStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            search_input: cx.new(|cx| InputState::new(window, cx).placeholder("Search...")),
            url_input: cx.new(|cx| InputState::new(window, cx).placeholder("example.com")),
            username_input: cx.new(|cx| InputState::new(window, cx).placeholder("@username")),
            chat_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Ask, Search or Chat...")
                    .multi_line(true)
            }),
        }
    }
}

impl Render for InputGroupStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("input-group-story")
            .size_full()
            .overflow_y_scroll()
            .gap_6()
            .child(
                section("Basic Search with Icon and Results").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Search).small()))
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new()
                                .inline_end()
                                .child(InputGroupText::new().child("12 results")),
                        ),
                ),
            )
            .child(
                section("URL Input with Protocol Prefix").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(
                            InputGroupAddon::new().child(InputGroupText::new().child("https://")),
                        )
                        .child(InputGroupInput::new(&self.url_input)),
                ),
            )
            .child(
                section("Username with Validation Icon").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupInput::new(&self.username_input))
                        .child(
                            InputGroupAddon::new().inline_end().child(
                                div()
                                    .size_4()
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(cx.theme().primary)
                                    .child(Icon::new(IconName::Check).text_sm()),
                            ),
                        ),
                ),
            )
            .child(
                section("Search with Clear Button").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Search).small()))
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new().inline_end().child(
                                Button::new("clear-btn")
                                    .xsmall()
                                    .ghost()
                                    .icon(IconName::Close)
                                    .rounded_full(),
                            ),
                        ),
                ),
            )
            .child(
                section("Search with Loading State").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Search).small()))
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new()
                                .inline_end()
                                .child(Spinner::new().small()),
                        ),
                ),
            )
            .child(
                section("Multiple Icons").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(
                            InputGroupAddon::new()
                                .child(Icon::new(IconName::Search).small())
                                .child(Icon::new(IconName::Settings).small()),
                        )
                        .child(InputGroupInput::new(&self.search_input)),
                ),
            )
            .child(
                section("With Action Button").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new().inline_end().child(
                                Button::new("send-btn")
                                    .xsmall()
                                    .primary()
                                    .label("Send")
                                    .rounded_full(),
                            ),
                        ),
                ),
            )
            .child(
                section("Disabled State").child(
                    InputGroup::new()
                        .max_w_96()
                        .disabled(true)
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Minus).small()))
                        .child(InputGroupInput::new(&self.search_input)),
                ),
            )
            .child(
                section("Invalid/Error State").child(
                    InputGroup::new()
                        .max_w_96()
                        .border_color(gpui::red())
                        .child(
                            InputGroupAddon::new()
                                .child(Icon::new(IconName::TriangleAlert).small()),
                        )
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new()
                                .inline_end()
                                .child(InputGroupText::new().child("Error!")),
                        ),
                ),
            )
            .child(
                section("Chat Input with Toolbar").child(
                    InputGroup::new()
                        .max_w_96()
                        .flex_col()
                        .h_auto()
                        .child(InputGroupTextarea::new(&self.chat_input).flex_1())
                        .child(
                            InputGroupAddon::new()
                                .block_end()
                                .child(
                                    Button::new("attach-btn")
                                        .xsmall()
                                        .ghost()
                                        .icon(IconName::Plus)
                                        .rounded_full(),
                                )
                                .child(Button::new("auto-btn").xsmall().ghost().label("Auto"))
                                .child(div().flex_1())
                                .child(InputGroupText::new().child("52% used"))
                                .child(div().h_4().w_px().bg(cx.theme().border))
                                .child(
                                    Button::new("send-btn")
                                        .xsmall()
                                        .primary()
                                        .icon(IconName::ArrowUp)
                                        .rounded_full()
                                        .disabled(true),
                                ),
                        ),
                ),
            )
    }
}
