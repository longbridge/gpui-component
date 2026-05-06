use crate::section;
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use gpui_component::{ActiveTheme, h_flex, label::Label, separator::Separator, v_flex};

const DESCRIPTION: &str = "GPUI Component is a Rust GUI components for building fantastic cross-platform desktop application by using GPUI.";

pub struct SeparatorStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for SeparatorStory {
    fn title() -> &'static str {
        "Separator"
    }

    fn description() -> &'static str {
        "A separator that can be either vertical or horizontal."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SeparatorStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Focusable for SeparatorStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SeparatorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Horizontal Separators").child(
                    v_flex()
                        .gap_4()
                        .w_full()
                        .mt_4()
                        .child(Separator::horizontal())
                        .child(Separator::horizontal().label("With Label"))
                        .child(Separator::horizontal_dashed())
                        .child(Separator::horizontal_dashed().label("Dashed With Label")),
                ),
            )
            .child(
                section("Vertical Separators").child(
                    h_flex()
                        .gap_4()
                        .h(px(100.))
                        .child(Separator::vertical())
                        .child(Separator::vertical().label("Solid"))
                        .child(Separator::vertical_dashed())
                        .child(Separator::vertical_dashed().label("Dashed")),
                ),
            )
            .child(
                section("Combination Separators").child(
                    v_flex()
                        .gap_y_4()
                        .child(
                            v_flex().gap_y_2().child("Hello GPUI Component").child(
                                Label::new(DESCRIPTION)
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm(),
                            ),
                        )
                        .child(Separator::horizontal())
                        .child(
                            h_flex()
                                .gap_x_4()
                                .child("Docs")
                                .child(Separator::vertical().dashed())
                                .child("Github")
                                .child(Separator::vertical().dashed())
                                .child("Source"),
                        ),
                ),
            )
    }
}
