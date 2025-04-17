use gpui::{
    px, App, AppContext, Entity, FocusHandle, Focusable, ParentElement as _, Render, Styled, Window,
};
use gpui_component::{dock::PanelControl, v_flex, SvgImg};

use crate::section;

const GOOGLE_LOGO: &str = include_str!("./fixtures/google.svg");

pub struct ImageStory {
    focus_handle: gpui::FocusHandle,
    google_logo: SvgImg,
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
            google_logo: SvgImg::new().source(GOOGLE_LOGO.as_bytes(), px(300.), px(300.)),
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
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex().gap_4().size_full().child(
            section("SVG Image")
                .child(self.google_logo.clone().size(px(100.)).flex_grow())
                .child(self.google_logo.clone().size(px(100.)).flex_grow())
                .child(self.google_logo.clone().size_80().flex_grow())
                .child(self.google_logo.clone().size_12().flex_grow())
                .child(self.google_logo.clone().size(px(100.))),
        )
    }
}
