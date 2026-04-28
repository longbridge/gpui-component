use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, px,
};
use gpui_component::{
    separator::{Orientation, Separator},
    v_flex,
};

use crate::section;

pub struct SeparatorStory {
    focus_handle: FocusHandle,
}

impl super::Story for SeparatorStory {
    fn title() -> &'static str {
        "Separator"
    }

    fn description() -> &'static str {
        "A simple horizontal/vertical separator."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SeparatorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for SeparatorStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SeparatorStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(section("Horizontal Separators").child(
                v_flex()
                    .gap_4()
                    .child(Separator::horizontal())
                    .child(Separator::horizontal().w(px(300.)))   // partial width
            ))
            .child(section("Vertical Separator").child(
                v_flex()
                    .h(px(100.))
                    .child(Separator::vertical())
            ))
    }
}
