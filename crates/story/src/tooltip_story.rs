use gpui::{
    div, App, AppContext, Context, CursorStyle, Entity, Focusable, InteractiveElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants},
    checkbox::Checkbox,
    dock::PanelControl,
    h_flex,
    label::Label,
    tooltip::Tooltip,
    v_flex, ActiveTheme, IconName,
};

pub struct TooltipStory {
    focus_handle: gpui::FocusHandle,
}

impl TooltipStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl super::Story for TooltipStory {
    fn title() -> &'static str {
        "Tooltip"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}
impl Focusable for TooltipStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}
impl Render for TooltipStory {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
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
                    .tooltip(|window, cx| Tooltip::new("This is a Button", window, cx)),
            )
            .child(
                h_flex()
                    .justify_center()
                    .cursor(CursorStyle::PointingHand)
                    .child(Label::new("Hover me"))
                    .id("tooltip-2")
                    .tooltip(|window, cx| Tooltip::new("This is a Label", window, cx)),
            )
            .child(
                div()
                    .cursor(CursorStyle::PointingHand)
                    .child(Checkbox::new("check").label("Remember me").checked(true))
                    .id("tooltip-3")
                    .tooltip(|window, cx| Tooltip::new("Checked!", window, cx)),
            )
            .child(
                div()
                    .cursor(CursorStyle::PointingHand)
                    .child(
                        Button::new("button")
                            .label("Hover me")
                            .with_variant(ButtonVariant::Primary),
                    )
                    .id("tooltip-4")
                    .tooltip(|window, cx| {
                        Tooltip::new_element(window, cx, |_, cx| {
                            h_flex()
                                .gap_x_1()
                                .child(IconName::Info)
                                .child(
                                    div()
                                        .child("Muted Foreground")
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(div().child("Danger").text_color(cx.theme().danger))
                                .child(IconName::ArrowUp)
                        })
                    }),
            )
    }
}
