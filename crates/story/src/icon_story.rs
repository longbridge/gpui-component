use gpui::{
    px, rems, ParentElement, Render, Styled, View, ViewContext, VisualContext as _, WindowContext,
};
use ui::{
    button::{Button, ButtonVariant, ButtonVariants},
    dock::PanelControl,
    h_flex, v_flex, ActiveTheme as _, Icon, IconName,
};

pub struct IconStory {
    focus_handle: gpui::FocusHandle,
}

impl IconStory {
    fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }
}

impl super::Story for IconStory {
    fn title() -> &'static str {
        "Icon"
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl gpui::FocusableView for IconStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for IconStory {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        v_flex().gap_3().child(
            h_flex()
                .gap_4()
                .child(IconName::Info)
                .child(
                    Icon::new(IconName::Maximize)
                        .size_6()
                        .text_color(ui::green_500()),
                )
                .child(Icon::new(IconName::Maximize).size(px(55.)))
                .child(
                    Button::new("like1")
                        .icon(
                            Icon::new(IconName::Heart)
                                .text_color(ui::gray_500())
                                .size_6(),
                        )
                        .with_variant(ButtonVariant::Ghost),
                )
                .child(
                    Button::new("like2")
                        .icon(
                            Icon::new(IconName::HeartOff)
                                .text_color(ui::red_500())
                                .size_6(),
                        )
                        .with_variant(ButtonVariant::Ghost),
                )
                .child(
                    Icon::new(IconName::Plus)
                        .w(rems(3.))
                        .h(rems(3.))
                        .bg(cx.theme().primary)
                        .text_color(cx.theme().primary_foreground)
                        .rounded(px(32.)),
                ),
        )
    }
}
