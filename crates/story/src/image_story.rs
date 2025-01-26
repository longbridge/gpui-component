use gpui::{
    px, App, AppContext, Entity, FocusHandle, Focusable, ParentElement as _, Render, Styled, Window,
};
use ui::{dock::PanelControl, h_flex, v_flex, SvgImg};

const GOOGLE_LOGO: &str = include_str!("./fixtures/google.svg");
const PIE_JSON: &str = include_str!("./fixtures/pie.json");

pub struct ImageStory {
    focus_handle: gpui::FocusHandle,
    google_logo: SvgImg,
    pie_chart: SvgImg,
    inbox_img: SvgImg,
}

impl super::Story for ImageStory {
    fn title() -> &'static str {
        "Image"
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
        let chart = charts_rs::PieChart::from_json(PIE_JSON).unwrap();

        Self {
            focus_handle: cx.focus_handle(),
            google_logo: SvgImg::new().source(GOOGLE_LOGO.as_bytes(), px(300.), px(300.)),
            pie_chart: SvgImg::new().source(chart.svg().unwrap().as_bytes(), px(600.), px(400.)),
            inbox_img: SvgImg::new().source("icons/inbox.svg", px(24.), px(24.)),
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
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .gap_4()
            .size_full()
            .items_start()
            .child(
                h_flex()
                    .size_full()
                    .child(self.google_logo.clone().size(px(300.)).flex_grow())
                    .child(self.google_logo.clone().size(px(300.)).flex_grow())
                    .child(self.google_logo.clone().size_80().flex_grow())
                    .child(self.google_logo.clone().size_12().flex_grow())
                    .child(self.google_logo.clone().size(px(300.))),
            )
            .child(self.inbox_img.clone().flex_shrink_0().size(px(64.)))
            .child(self.pie_chart.clone().flex_shrink_0().w_full().h(px(400.)))
    }
}
