use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    empty::Empty,
    v_flex, IconName,
};

use crate::section;

pub struct EmptyStory {
    focus_handle: FocusHandle,
}

impl super::Story for EmptyStory {
    fn title() -> &'static str {
        "Empty"
    }

    fn description() -> &'static str {
        "Placeholder for empty states."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl EmptyStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for EmptyStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EmptyStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(section("Minimal").child(
                Empty::new()
                    .title("No items")
                    .description("Your list is empty."),
            ))
            .child(section("With icon and action").child(
                Empty::new()
                    .icon(IconName::Inbox)
                    .title("Inbox zero!")
                    .description("You have no new messages.")
                    .action(
                        Button::new("refresh")
                            .primary()
                            .label("Refresh"),
                    ),
            ))
    }
}
