use crate::section;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Image, ImageFormat, IntoElement,
    ParentElement as _, Render, Styled, Window, img,
};
use gpui_component::{dock::PanelControl, v_flex};
use std::sync::{Arc, LazyLock};

macro_rules! include_svg {
    ($file:expr $(,)?) => {{
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            include_bytes!($file).into(),
        ))
    }};
}

static IMAGE_GOOGLE: LazyLock<Arc<Image>> = LazyLock::new(|| include_svg!("fixtures/google.svg"));
static IMAGE_COLOR_WHEEL: LazyLock<Arc<Image>> =
    LazyLock::new(|| include_svg!("fixtures/color-wheel.svg"));

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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
        let google = IMAGE_GOOGLE.clone();
        let color_wheel = IMAGE_COLOR_WHEEL.clone();

        v_flex()
            .gap_4()
            .size_full()
            .child(section("SVG 160px").child(img(google).size_40().flex_grow()))
            .child(section("SVG 80px").child(img(color_wheel).size_20().flex_grow()))
            .child(
                section("SVG from img 40px").child(
                    img("https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg").h_24(),
                ),
            )
    }
}
