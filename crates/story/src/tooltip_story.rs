use gpui::{
    div, CursorStyle, InteractiveElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, View, ViewContext, VisualContext as _, WindowContext,
};

use ui::{
    button::{Button, ButtonVariant, ButtonVariants},
    checkbox::Checkbox,
    dock::ToolbarType,
    h_flex,
    label::Label,
    tooltip::Tooltip,
    v_flex,
};

pub struct TooltipStory {
    focus_handle: gpui::FocusHandle,
}

impl TooltipStory {
    pub fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }

    fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl super::Story for TooltipStory {
    fn title() -> &'static str {
        "Tooltip"
    }

    fn new_view(cx: &mut WindowContext) -> View<impl gpui::FocusableView> {
        Self::view(cx)
    }

    fn zoomable() -> Option<ToolbarType> {
        None
    }
}
impl gpui::FocusableView for TooltipStory {
    fn focus_handle(&self, _: &gpui::AppContext) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TooltipStory {
    fn render(&mut self, _: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(
                div()
                    .cursor(CursorStyle::PointingHand)
                    .child(
                        Button::new("button")
                            .label("Hover me")
                            .with_variant(ButtonVariant::Primary),
                    )
                    .id("tooltip-1")
                    .tooltip(|cx| Tooltip::new("This is a Button", cx)),
            )
            .child(
                h_flex()
                    .justify_center()
                    .cursor(CursorStyle::PointingHand)
                    .child(Label::new("Hover me"))
                    .id("tooltip-3")
                    .tooltip(|cx| Tooltip::new("This is a Label", cx)),
            )
            .child(
                div()
                    .cursor(CursorStyle::PointingHand)
                    .child(Checkbox::new("check").label("Remember me").checked(true))
                    .id("tooltip-4")
                    .tooltip(|cx| Tooltip::new("Checked!", cx)),
            )
    }
}
