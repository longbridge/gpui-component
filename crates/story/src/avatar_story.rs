use gpui::{
    px, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window,
};
use gpui_component::{
    avatar::{Avatar, AvatarGroup},
    dock::PanelControl,
    v_flex, ActiveTheme, IconName, Sizable as _, StyledExt,
};

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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Avatar with image")
                    .max_w_md()
                    .child(
                        Avatar::new()
                            .name("Jason lee")
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
                    .child(Avatar::new().name("Jason Lee").large())
                    .child(Avatar::new().name("Floyd Wang"))
                    .child(Avatar::new().name("xda").small())
                    .child(Avatar::new().name("ihavecoke").xsmall()),
            )
            .child(
                section("Placeholder")
                    .max_w_md()
                    .child(Avatar::new().large())
                    .child(Avatar::new())
                    .child(Avatar::new().small())
                    .child(Avatar::new().xsmall())
                    .child(Avatar::new().placeholder(IconName::Building2)),
            )
            .child(
                section("Avatar Group")
                    .v_flex()
                    .max_w_md()
                    .child(
                        AvatarGroup::new()
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=a"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=b"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=c"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=d"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=e"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=f")),
                    )
                    .child(
                        AvatarGroup::new()
                            .small()
                            .limit(5)
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=a"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=b"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=c"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=d"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=e"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=f"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=g"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=h"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=i"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=j"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=k")),
                    )
                    .child(
                        AvatarGroup::new()
                            .xsmall()
                            .limit(6)
                            .ellipsis()
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=a"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=b"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=c"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=d"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=e"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=f"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=g"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=h"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=i"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=j"))
                            .child(Avatar::new().src("https://i.pravatar.cc/200?u=k")),
                    ),
            )
            .child(
                section("Custom rounded").child(
                    Avatar::new()
                        .src("https://i.pravatar.cc/200?u=a")
                        .with_size(px(100.))
                        .rounded(px(20.)),
                ),
            )
            .child(
                section("Custom Style").child(
                    Avatar::new()
                        .src("https://i.pravatar.cc/200?u=c")
                        .with_size(px(100.))
                        .border_3()
                        .border_color(cx.theme().foreground)
                        .shadow_sm(),
                ),
            )
    }
}
