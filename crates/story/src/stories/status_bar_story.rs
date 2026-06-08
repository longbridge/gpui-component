use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    dock::PanelControl,
    status_bar::{StatusBar, StatusBarItem},
    v_flex, IconName,
};

use crate::section;

pub struct StatusBarStory {
    focus_handle: gpui::FocusHandle,
    clicked: usize,
}

impl StatusBarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            clicked: 0,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for StatusBarStory {
    fn title() -> &'static str {
        "StatusBar"
    }

    fn description() -> &'static str {
        "A horizontal bar with left/center/right regions, usually placed at the bottom."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for StatusBarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StatusBarStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Left and right").child(
                    StatusBar::new()
                        .left(StatusBarItem::new("status").label("Ready"))
                        .right(StatusBarItem::new("encoding").label("UTF-8")),
                ),
            )
            .child(
                section("Three regions").child(
                    StatusBar::new()
                        .left(StatusBarItem::new("branch").icon(IconName::Github).label("main"))
                        .center(StatusBarItem::new("title").label("README.md"))
                        .right(StatusBarItem::new("position").label("Ln 1, Col 1")),
                ),
            )
            .child(
                section("With icons").child(
                    StatusBar::new()
                        .left(StatusBarItem::new("info").icon(IconName::Info).label("12 issues"))
                        .left(StatusBarItem::new("bell").icon(IconName::Bell).label("3"))
                        .right(StatusBarItem::new("lang").icon(IconName::Globe).label("Rust")),
                ),
            )
            .child(
                section("Interactive vs. read-only").child(
                    StatusBar::new()
                        .left(
                            StatusBarItem::new("clickable")
                                .icon(IconName::CircleCheck)
                                .label(format!("Clicked: {}", self.clicked))
                                .tooltip("Click me")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.clicked += 1;
                                    cx.notify();
                                })),
                        )
                        .right(StatusBarItem::new("readonly").label("Read-only")),
                ),
            )
    }
}
