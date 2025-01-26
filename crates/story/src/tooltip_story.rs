use gpui::{Window, ModelContext, AppContext, Model, 
    div, CursorStyle, InteractiveElement, ParentElement, Render, StatefulInteractiveElement,
    Styled,   VisualContext as _, 
};

use ui::{
    button::{Button, ButtonVariant, ButtonVariants},
    checkbox::Checkbox,
    dock::PanelControl,
    h_flex,
    label::Label,
    tooltip::Tooltip,
    v_flex,
};

pub struct TooltipStory {
    focus_handle: gpui::FocusHandle,
}

impl TooltipStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(Self::new)
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl super::Story for TooltipStory {
    fn title() -> &'static str {
        "Tooltip"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl gpui::Focusable> {
        Self::view(cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}
impl gpui::Focusable for TooltipStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TooltipStory {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
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
