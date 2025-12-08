use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, Keystroke, ParentElement, Render,
    Styled, Window,
};

use gpui_component::{h_flex, kbd::Kbd, v_flex};

use crate::section;

pub struct KbdStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for KbdStory {
    fn title() -> &'static str {
        "Kbd"
    }

    fn description() -> &'static str {
        "A tag style to display keyboard shortcuts"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl KbdStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}
impl Focusable for KbdStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for KbdStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_6().child(
            section("Kbd").child(
                h_flex()
                    .gap_2()
                    .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
                    .child(Kbd::new(Keystroke::parse("cmd-ctrl-t").unwrap()))
                    .child(Kbd::new(Keystroke::parse("cmd--").unwrap()))
                    .child(Kbd::new(Keystroke::parse("cmd-+").unwrap()))
                    .child(Kbd::new(Keystroke::parse("escape").unwrap()))
                    .child(Kbd::new(Keystroke::parse("backspace").unwrap()))
                    .child(Kbd::new(Keystroke::parse("/").unwrap()))
                    .child(Kbd::new(Keystroke::parse("enter").unwrap())),
            ),
        )
    }
}
