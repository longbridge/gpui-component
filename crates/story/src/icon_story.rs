use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants},
    dock::PanelControl,
    gray_500, green_500, h_flex, red_500, v_flex, Icon, IconName,
};

use crate::section;

pub struct IconStory {
    focus_handle: gpui::FocusHandle,
}

impl IconStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl super::Story for IconStory {
    fn title() -> &'static str {
        "Icon"
    }

    fn description() -> &'static str {
        "SVG Icons based on Lucide.dev"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for IconStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IconStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                section("Icon")
                    .text_lg()
                    .child(IconName::Info)
                    .child(IconName::Map)
                    .child(IconName::Bot)
                    .child(IconName::GitHub)
                    .child(IconName::Calendar)
                    .child(IconName::Globe)
                    .child(IconName::Heart),
            )
            .child(
                section("Color Icon")
                    .child(
                        Icon::new(IconName::Maximize)
                            .size_6()
                            .text_color(green_500()),
                    )
                    .child(Icon::new(IconName::Minimize).size_6().text_color(red_500())),
            )
            .child(
                section("Icon Button").child(
                    h_flex()
                        .gap_4()
                        .child(
                            Button::new("like1")
                                .icon(Icon::new(IconName::Heart).text_color(gray_500()).size_6())
                                .with_variant(ButtonVariant::Ghost),
                        )
                        .child(
                            Button::new("like2")
                                .icon(Icon::new(IconName::HeartOff).text_color(red_500()).size_6())
                                .with_variant(ButtonVariant::Ghost),
                        )
                        .child(
                            Button::new("like3")
                                .icon(Icon::new(IconName::Heart).text_color(green_500()).size_6())
                                .with_variant(ButtonVariant::Ghost),
                        ),
                ),
            )
    }
}
