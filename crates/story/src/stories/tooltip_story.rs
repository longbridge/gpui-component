use gpui::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, KeyBinding, ParentElement,
    Render, StatefulInteractiveElement as _, Styled, Window, actions, div,
};

use gpui_component::{
    IconName,
    button::{Button, ButtonVariant, ButtonVariants, Toggle},
    checkbox::Checkbox,
    clipboard::Clipboard,
    dock::PanelControl,
    h_flex,
    radio::Radio,
    switch::Switch,
    tooltip::Tooltip,
    v_flex,
};

use crate::{Story, section};

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

    fn description() -> &'static str {
        "A popup that displays information related to an element when the element receives keyboard focus or the mouse hovers over it."
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
            .w_full()
            .gap_3()
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
                        Button::new("btn3")
                            .label("Hover me")
                            .tooltip("This is tooltip 3"),
                    ),
            )
            .child(
                section("Checkbox Tooltip").child(
                    Checkbox::new("check")
                        .label("Remember me")
                        .checked(true)
                        .tooltip("This is a tooltip"),
                ),
            )
            .child(
                section("Radio Tooltip").child(
                    Radio::new("radio")
                        .label("Radio with tooltip")
                        .checked(true)
                        .tooltip("This is a radio button"),
                ),
            )
            .child(
                section("Switch Tooltip").child(
                    Switch::new("switch")
                        .checked(true)
                        .tooltip("This is a switch"),
                ),
            )
            .child(
                section("Toggle Tooltip").child(
                    h_flex()
                        .gap_2()
                        .child(Toggle::new("toggle1").label("Bold").tooltip("Toggle bold"))
                        .child(
                            Toggle::new("toggle2")
                                .icon(IconName::Heart)
                                .tooltip("Toggle favorite"),
                        ),
                ),
            )
            .child(
                section("Clipboard Tooltip").child(
                    Clipboard::new("clip1")
                        .value("Hello, World!")
                        .tooltip("Copy to clipboard"),
                ),
            )
            .child(
                section("Default Tooltip").child(div().child("Hover me").id("tooltip-2").tooltip(
                    |window, cx| {
                        Tooltip::new("This is a default tooltip style by GPUI.")
                            .action(&Info, Some("Tooltip"))
                            .build(window, cx)
                    },
                )),
            )
    }
}
