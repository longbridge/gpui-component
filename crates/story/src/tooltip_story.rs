use gpui::{
    actions, div, App, AppContext, Context, Entity, Focusable, InteractiveElement, KeyBinding,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants},
    checkbox::Checkbox,
    dock::PanelControl,
    h_flex,
    radio::Radio,
    switch::Switch,
    tooltip::Tooltip,
    v_flex, ActiveTheme, IconName,
};

use crate::{section, Story};

actions!(tooltip_story, [Info]);

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-delete", Info, Some("Tooltip"))]);
}

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

impl Story for TooltipStory {
    fn title() -> &'static str {
        "Tooltip"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
        _: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .p_4()
            .gap_5()
            .child(
                section("Tooltip for Button")
                    .child(
                        Button::new("btn0")
                            .label("Search")
                            .with_variant(ButtonVariant::Primary)
                            .tooltip("This is a search Button."),
                    )
                    .child(Button::new("btn1").label("Info").tooltip_with_action(
                        "This is a tooltip with Action for display keybinding.",
                        &Info,
                        Some("Tooltip"),
                    ))
                    .child(
                        div()
                            .child(Button::new("btn3").label("Hover me"))
                            .id("tooltip-4")
                            .tooltip(|window, cx| {
                                Tooltip::element(|_, cx| {
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
                                .build(window, cx)
                            }),
                    ),
            )
            .child(
                section("Label Tooltip").child(div().child("Hover me").id("tooltip-2").tooltip(
                    |window, cx| {
                        Tooltip::new("This is a Label")
                            .action(&Info, Some("Tooltip"))
                            .build(window, cx)
                    },
                )),
            )
            .child(
                section("Checkbox Tooltip").child(
                    Checkbox::new("check")
                        .label("Remember me")
                        .checked(true)
                        .tooltip(|window, cx| Tooltip::new("This is a checkbox").build(window, cx)),
                ),
            )
            .child(
                section("Radio Tooltip").child(
                    Radio::new("radio")
                        .label("Radio with tooltip")
                        .checked(true)
                        .tooltip(|window, cx| {
                            Tooltip::new("This is a radio button").build(window, cx)
                        }),
                ),
            )
            .child(
                section("Switch Tooltip").child(
                    Switch::new("switch")
                        .checked(true)
                        .tooltip("This is a switch"),
                ),
            )
    }
}
