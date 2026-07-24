use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    kbd::Kbd,
    kbd_group::KbdGroup,
    v_flex,
};

use crate::section;

pub struct KbdGroupStory {
    focus_handle: FocusHandle,
}

impl super::Story for KbdGroupStory {
    fn title() -> &'static str {
        "KbdGroup"
    }

    fn description() -> &'static str {
        "Display grouped keyboard shortcuts."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl KbdGroupStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for KbdGroupStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KbdGroupStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(section("Basic").child(
                KbdGroup::new()
                    .child(Kbd::new(gpui::Keystroke::parse("cmd-c").unwrap()))
                    .child(Kbd::new(gpui::Keystroke::parse("ctrl-v").unwrap())),
            ))
            .child(section("Three keys").child(
                KbdGroup::new()
                    .child(Kbd::new(gpui::Keystroke::parse("cmd-shift-p").unwrap()))
                    .child(Kbd::new(gpui::Keystroke::parse("cmd-ctrl-t").unwrap()))
                    .child(Kbd::new(gpui::Keystroke::parse("escape").unwrap())),
            ))
    }
}
