use crate::section;
use gpui::{
    img, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, Styled, Window,
};
use gpui_component::{dock::PanelControl, v_flex};

pub struct ImageStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for ImageStory {
    fn title() -> &'static str {
        "Image"
    }

    fn description() -> &'static str {
        "Image and SVG image supported."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::Toolbar)
    }
}

impl ImageStory {
    pub fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Focusable for ImageStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ImageStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // The svg file are from Assets
        // See: crates/story/src/assets.rs#L21
        v_flex()
            .gap_4()
            .size_full()
            .child(section("SVG 160px").child(img("src/fixtures/google.svg").size_40().flex_grow()))
            .child(
                section("SVG 80px")
                    .child(img("src/fixtures/color-wheel.svg").size_20().flex_grow()),
            )
            .child(
                section("SVG from img 40px").child(
                    img("https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg").h_24(),
                ),
            )
    }
}
