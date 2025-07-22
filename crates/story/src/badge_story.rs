use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    avatar::Avatar, badge::Badge, dock::PanelControl, v_flex, ActiveTheme as _, Icon, IconName,
    Sizable as _,
};

use crate::section;

pub struct BadgeStory {
    focus_handle: gpui::FocusHandle,
}

impl BadgeStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for BadgeStory {
    fn title() -> &'static str {
        "Badge"
    }

    fn description() -> &'static str {
        "A red dot that indicates the number of unread messages."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for BadgeStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BadgeStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Badge on icon")
                    .max_w_md()
                    .child(
                        Badge::new()
                            .count(3)
                            .child(Icon::new(IconName::Bell).large()),
                    )
                    .child(
                        Badge::new()
                            .count(103)
                            .child(Icon::new(IconName::Inbox).large()),
                    ),
            )
            .child(
                section("Badge with count")
                    .max_w_md()
                    .child(Badge::new().count(3).child(
                        Avatar::new().src("https://avatars.githubusercontent.com/u/5518?v=4"),
                    ))
                    .child(Badge::new().count(103).child(
                        Avatar::new().src("https://avatars.githubusercontent.com/u/28998859?v=4"),
                    )),
            )
            .child(
                section("Badge with icon")
                    .max_w_md()
                    .child(
                        Badge::new()
                            .icon(IconName::Check)
                            .color(cx.theme().cyan)
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                            ),
                    )
                    .child(
                        Badge::new()
                            .icon(IconName::Star)
                            .color(cx.theme().yellow)
                            .child(
                                Avatar::new()
                                    .src("https://avatars.githubusercontent.com/u/20092316?v=4"),
                            ),
                    ),
            )
            .child(
                section("Badge with dot").max_w_md().child(
                    Badge::new().dot().count(1).child(
                        Avatar::new().src("https://avatars.githubusercontent.com/u/5518?v=4"),
                    ),
                ),
            )
            .child(
                section("Badge with color")
                    .max_w_md()
                    .child(Badge::new().count(3).color(cx.theme().blue).child(
                        Avatar::new().src("https://avatars.githubusercontent.com/u/5518?v=4"),
                    ))
                    .child(Badge::new().dot().color(cx.theme().green).count(1).child(
                        Avatar::new().src("https://avatars.githubusercontent.com/u/5518?v=4"),
                    )),
            )
            .child(
                section("Complex use")
                    .max_w_md()
                    .child(
                        Badge::new().count(212).large().child(
                            Badge::new()
                                .icon(IconName::Check)
                                .large()
                                .color(cx.theme().cyan)
                                .child(
                                    Avatar::new()
                                        .large()
                                        .src("https://avatars.githubusercontent.com/u/5518?v=4"),
                                ),
                        ),
                    )
                    .child(
                        Badge::new().count(2).color(cx.theme().green).large().child(
                            Badge::new()
                                .icon(IconName::Star)
                                .large()
                                .color(cx.theme().yellow)
                                .child(
                                    Avatar::new().large().src(
                                        "https://avatars.githubusercontent.com/u/20092316?v=4",
                                    ),
                                ),
                        ),
                    )
                    .child(
                        Badge::new().count(3).color(cx.theme().green).child(
                            Badge::new()
                                .icon(IconName::Asterisk)
                                .color(cx.theme().green)
                                .child(
                                    Avatar::new().src(
                                        "https://avatars.githubusercontent.com/u/22312482?v=4",
                                    ),
                                ),
                        ),
                    )
                    .child(
                        Badge::new().dot().child(
                            Badge::new()
                                .icon(IconName::Sun)
                                .small()
                                .color(cx.theme().red)
                                .child(
                                    Avatar::new().small().src(
                                        "https://avatars.githubusercontent.com/u/150917089?v=4",
                                    ),
                                ),
                        ),
                    ),
            )
    }
}
