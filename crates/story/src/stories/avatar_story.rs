use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, px,
};
use gpui_component::{
    ActiveTheme, IconName, Sizable as _, StyledExt,
    avatar::{Avatar, AvatarGroup},
    dock::PanelControl,
    v_flex,
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
                            .src("https://avatars.githubusercontent.com/u/5518?v=4")
                            .with_size(px(100.)),
                    )
                    .child(
                        Avatar::new()
                            .src("https://avatars.githubusercontent.com/u/28998859?v=4")
                            .large(),
                    )
                    .child(
                        Avatar::new()
                            .src("https://avatars.githubusercontent.com/u/10757551?s=64&v=4"),
                    )
                    .child(
                        Avatar::new()
                            .src("https://avatars.githubusercontent.com/u/20092316?v=4")
                            .small(),
                    )
                    .child(
                        Avatar::new()
                            .src("https://avatars.githubusercontent.com/u/150917089?v=4")
                            .xsmall(),
                    ),
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
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/28998859?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/20092316?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/22312482?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/150917089?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/1253486?v=4"),
                            ),
                    )
                    .child(
                        AvatarGroup::new()
                            .small()
                            .limit(5)
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/28998859?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/20092316?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/22312482?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/150917089?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/20337280?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/629429?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/583231?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/1264109?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/2936367?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/1253486?v=4"),
                            ),
                    )
                    .child(
                        AvatarGroup::new()
                            .xsmall()
                            .limit(6)
                            .ellipsis()
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/28998859?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/20092316?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/22312482?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/150917089?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/20337280?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/2936367?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/583231?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/1264109?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/10757551?v=4"),
                            )
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/1506323?v=4"),
                            ),
                    ),
            )
            .child(
                section("Custom rounded").child(
                    Avatar::new()
                        .src("https://avatars.githubusercontent.com/u/5518?v=4")
                        .with_size(px(100.))
                        .rounded(px(20.)),
                ),
            )
            .child(
                section("Custom Style").child(
                    Avatar::new()
                        .src("https://avatars.githubusercontent.com/u/20092316?v=4")
                        .with_size(px(100.))
                        .border_3()
                        .border_color(cx.theme().foreground)
                        .shadow_sm(),
                ),
            )
    }
}
