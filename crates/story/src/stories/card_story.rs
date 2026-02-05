use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonVariants},
    card::Card,
    input::{Input, InputState},
    v_flex,
};

use crate::section;

pub struct CardStory {
    focus_handle: FocusHandle,
    input_account: Entity<InputState>,
    input_password: Entity<InputState>,
}

impl super::Story for CardStory {
    fn title() -> &'static str {
        "Card"
    }

    fn description() -> &'static str {
        "A card is a container that can be used to display content in a styled way."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CardStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            // QA: focus_handle 是什么作用？
            focus_handle: cx.focus_handle(),
            input_account: cx
                .new(|cx| InputState::new(window, cx).placeholder("Enter your account")),
            input_password: cx
                .new(|cx| InputState::new(window, cx).placeholder("Enter your password")),
        }
    }
}

// QA: 这个 focus_handle 是什么作用？
impl Focusable for CardStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CardStory {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_6().child(
            section("Normal Card").max_w_full().child(
                Card::new("card_1")
                    .title("This is the card title.")
                    .child("This is the card content. \nLorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."),
            ),
        ).child(
            section("Card with Footer").max_w_full().child(
                Card::new("card_2")
                    .title("Login to your account")
                    .child(
                        v_flex()
                        .gap_2()
                        .child(Input::new(&self.input_account))
                        .child(Input::new(&self.input_password))
                    )
                    .footer(
                v_flex()
                            .gap_2()
                            .child(Button::new("button_1").primary().label("Login"))
                            .child(Button::new("button_2").outline().label("Sign up"))
                    ),
            ),
        )
    }
}
