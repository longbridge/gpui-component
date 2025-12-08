use crate::section;
use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use gpui_component::{ActiveTheme, divider::Divider, h_flex, label::Label, v_flex};

const DESCRIPTION: &str = "GPUI Component is a Rust GUI components for building fantastic cross-platform desktop application by using GPUI.";

pub struct DividerStory {
    focus_handle: gpui::FocusHandle,
}

impl super::Story for DividerStory {
    fn title() -> &'static str {
        "Divider"
    }

    fn description() -> &'static str {
        "A divider that can be either vertical or horizontal."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DividerStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Focusable for DividerStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DividerStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                section("Horizontal Dividers").child(
                    v_flex()
                        .gap_4()
                        .w_full()
                        .mt_4()
                        .child(Divider::horizontal())
                        .child(Divider::horizontal().label("With Label"))
                        .child(Divider::horizontal_dashed())
                        .child(Divider::horizontal_dashed().label("Dashed With Label")),
                ),
            )
            .child(
                section("Vertical Dividers").child(
                    h_flex()
                        .gap_4()
                        .h(px(100.))
                        .child(Divider::vertical())
                        .child(Divider::vertical().label("Solid"))
                        .child(Divider::vertical_dashed())
                        .child(Divider::vertical_dashed().label("Dashed")),
                ),
            )
            .child(
                section("Combination Dividers").child(
                    v_flex()
                        .gap_y_4()
                        .child(
                            v_flex().gap_y_2().child("Hello GPUI Component").child(
                                Label::new(DESCRIPTION)
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm(),
                            ),
                        )
                        .child(Divider::horizontal())
                        .child(
                            h_flex()
                                .gap_x_4()
                                .child("Docs")
                                .child(Divider::vertical().dashed())
                                .child("Github")
                                .child(Divider::vertical().dashed())
                                .child("Source"),
                        ),
                ),
            )
    }
}
