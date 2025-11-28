use gpui::{
    div, App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled, Window,
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
                    .multi_line()
            }),
        }
    }

    // === Helper Functions ===

    /// Create a standard InputGroup with max-width
    fn example_group() -> InputGroup {
        InputGroup::new().max_w_96()
    }

    /// Create an addon with a small icon
    fn icon_addon(icon: IconName) -> InputGroupAddon {
        InputGroupAddon::new().child(Icon::new(icon).small())
    }

    /// Create a right-aligned addon
    fn end_addon() -> InputGroupAddon {
        InputGroupAddon::new().align(InputGroupAlign::InlineEnd)
    }

    /// Create a bottom-aligned addon
    fn bottom_addon() -> InputGroupAddon {
        InputGroupAddon::new().align(InputGroupAlign::BlockEnd)
    }

    /// Create a small icon button
    fn icon_button(id: impl Into<ElementId>, icon: IconName) -> Button {
        Button::new(id)
            .xsmall()
            .ghost()
            .icon(icon)
            .rounded_full()
    }

    /// Create a primary icon button
    fn primary_icon_button(id: impl Into<ElementId>, icon: IconName) -> Button {
        Button::new(id)
            .xsmall()
            .primary()
            .icon(icon)
            .rounded_full()
    }

    /// Create a vertical separator
    fn separator(cx: &Context<Self>) -> impl IntoElement {
        div().h_4().w_px().bg(cx.theme().border)
    }

    /// Create a validation badge (check mark)
    fn validation_badge(cx: &Context<Self>) -> impl IntoElement {
        div()
            .size_4()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().primary)
            .child(Icon::new(IconName::Check).text_sm())
    }

    // === Example Renderers ===

    fn render_basic_search(&self) -> impl IntoElement {
        section("Basic Search with Icon and Results").child(
            Self::example_group()
                .child(Self::icon_addon(IconName::Search))
                .child(InputGroupInput::new(&self.search_input))
                .child(Self::end_addon().child(InputGroupText::label("12 results"))),
        )
    }

    fn render_url_input(&self) -> impl IntoElement {
        section("URL Input with Protocol Prefix").child(
            Self::example_group()
                .child(InputGroupAddon::new().child(InputGroupText::label("https://")))
                .child(InputGroupInput::new(&self.url_input)),
        )
    }

    fn render_username_validation(&self, cx: &Context<Self>) -> impl IntoElement {
        section("Username with Validation Icon").child(
            Self::example_group()
                .child(InputGroupInput::new(&self.username_input))
                .child(Self::end_addon().child(Self::validation_badge(cx))),
        )
    }

    fn render_search_with_clear(&self) -> impl IntoElement {
        section("Search with Clear Button").child(
            Self::example_group()
                .child(Self::icon_addon(IconName::Search))
                .child(InputGroupInput::new(&self.search_input))
                .child(Self::end_addon().child(Self::icon_button("clear-btn", IconName::Close))),
        )
    }

    fn render_search_loading(&self) -> impl IntoElement {
        section("Search with Loading State").child(
            Self::example_group()
                .child(Self::icon_addon(IconName::Search))
                .child(InputGroupInput::new(&self.search_input))
                .child(Self::end_addon().child(Spinner::new().small())),
        )
    }

    fn render_multiple_icons(&self) -> impl IntoElement {
        section("Multiple Icons").child(
            Self::example_group()
                .child(
                    InputGroupAddon::new()
                        .child(Icon::new(IconName::Search).small())
                        .child(Icon::new(IconName::Settings).small()),
                )
                .child(InputGroupInput::new(&self.search_input)),
        )
    }

    fn render_with_action_button(&self) -> impl IntoElement {
        section("With Action Button").child(
            Self::example_group()
                .child(InputGroupInput::new(&self.search_input))
                .child(
                    Self::end_addon().child(
                        Button::new("send-btn")
                            .xsmall()
                            .primary()
                            .label("Send")
                            .rounded_full(),
                    ),
                ),
        )
    }

    fn render_disabled_state(&self) -> impl IntoElement {
        section("Disabled State").child(
            Self::example_group()
                .disabled(true)
                .child(Self::icon_addon(IconName::Minus))
                .child(InputGroupInput::new(&self.search_input)),
        )
    }

    fn render_error_state(&self) -> impl IntoElement {
        section("Invalid/Error State").child(
            Self::example_group()
                .invalid(true)
                .child(Self::icon_addon(IconName::TriangleAlert))
                .child(InputGroupInput::new(&self.search_input))
                .child(Self::end_addon().child(InputGroupText::label("Error!"))),
        )
    }

    fn render_chat_input(&self, cx: &Context<Self>) -> impl IntoElement {
        section("Chat Input with Toolbar").child(
            Self::example_group()
                .flex_col()
                .h_auto()
                .child(InputGroupTextarea::new(&self.chat_input).flex_1())
                .child(
                    Self::bottom_addon()
                        // Left side buttons
                        .child(Self::icon_button("attach-btn", IconName::Plus))
                        .child(Button::new("auto-btn").xsmall().ghost().label("Auto"))
                        // Spacer to push right side content
                        .child(div().flex_1())
                        // Right side content
                        .child(InputGroupText::label("52% used"))
                        .child(Self::separator(cx))
                        .child(Self::primary_icon_button("send-btn", IconName::ArrowUp).disabled(true)),
                ),
        )
    }
}

impl Render for InputGroupStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("input-group-story")
            .size_full()
            .overflow_y_scroll()
            .gap_6()
            .child(self.render_basic_search())
            .child(self.render_url_input())
            .child(self.render_username_validation(cx))
            .child(self.render_search_with_clear())
            .child(self.render_search_loading())
            .child(self.render_multiple_icons())
            .child(self.render_with_action_button())
            .child(self.render_disabled_state())
            .child(self.render_error_state())
            .child(self.render_chat_input(cx))
    }
}
