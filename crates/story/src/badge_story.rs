use gpui::{
    img, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window,
};
use gpui_component::{
    badge::Badge, dock::PanelControl, v_flex, ActiveTheme, Icon, IconName, Sizable as _,
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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
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
                section("Badge on Icon")
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
                section("Badge on Avatar")
                    .max_w_md()
                    .child(
                        Badge::new().count(3).child(
                            img("https://avatars.githubusercontent.com/u/5518?v=4")
                                .size_10()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_full(),
                        ),
                    )
                    .child(
                        Badge::new().count(103).child(
                            img("https://avatars.githubusercontent.com/u/28998859?v=4")
                                .size_10()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_full(),
                        ),
                    ),
            )
    }
}
