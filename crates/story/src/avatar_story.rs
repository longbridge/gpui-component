use gpui::{
    px, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window,
};
use gpui_component::{avatar::Avatar, dock::PanelControl, v_flex, IconName, Sizable as _};

use crate::section;

pub struct AvatarStory {
    focus_handle: gpui::FocusHandle,
}

impl AvatarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for AvatarStory {
    fn title() -> &'static str {
        "Avatar"
    }

    fn description() -> &'static str {
        "Avatar is an image that represents a user or organization."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for AvatarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AvatarStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Avatar with image")
                    .max_w_md()
                    .child(
                        Avatar::new()
                            .src("https://i.pravatar.cc/200?u=a")
                            .with_size(px(100.)),
                    )
                    .child(Avatar::new().src("https://i.pravatar.cc/200?u=b").large())
                    .child(Avatar::new().src("https://i.pravatar.cc/200?u=c"))
                    .child(Avatar::new().src("https://i.pravatar.cc/200?u=d").small())
                    .child(Avatar::new().src("https://i.pravatar.cc/200?u=e").xsmall()),
            )
            .child(
                section("Avatar with text")
                    .max_w_md()
                    .child(Avatar::new().text("Jason Lee").large())
                    .child(Avatar::new().text("Floyd Wang"))
                    .child(Avatar::new().text("xda").small())
                    .child(Avatar::new().text("ihavecoke").xsmall()),
            )
            .child(
                section("Placeholder")
                    .max_w_md()
                    .child(Avatar::new())
                    .child(Avatar::new().placeholder(IconName::Building2)),
            )
            .child(
                section("Custom rounded").child(
                    Avatar::new()
                        .src("https://i.pravatar.cc/200?u=a")
                        .with_size(px(100.))
                        .rounded(px(20.)),
                ),
            )
    }
}
