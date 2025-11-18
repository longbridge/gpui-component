use gpui::{
    Action, App, AppContext as _, Context, Corner, Entity, Focusable, IntoElement,
    ParentElement as _, Render, Styled as _, Window, prelude::FluentBuilder as _,
};
use serde::Deserialize;

use crate::section;
use gpui_component::{
    ActiveTheme, Disableable, Selectable as _, Sizable as _, Theme,
    button::{Button, ButtonVariants as _, DropdownButton},
    checkbox::Checkbox,
    h_flex, v_flex,
};

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = dropdown_button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
}

pub struct DropdownButtonStory {
    focus_handle: gpui::FocusHandle,
    disabled: bool,
    loading: bool,
    selected: bool,
    compact: bool,
}

impl DropdownButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            loading: false,
            selected: false,
            compact: false,
        })
    }
}

impl super::Story for DropdownButtonStory {
    fn title() -> &'static str {
        "DropdownButton"
    }

    fn description() -> &'static str {
        "A button with an attached dropdown menu for additional options."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for DropdownButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DropdownButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.disabled;
        let loading = self.loading;
        let selected = self.selected;
        let compact = self.compact;

        v_flex()
            .gap_6()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Checkbox::new("disabled-button")
                            .label("Disabled")
                            .checked(self.disabled)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.disabled = !view.disabled;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("loading-button")
                            .label("Loading")
                            .checked(self.loading)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.loading = !view.loading;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("selected-button")
                            .label("Selected")
                            .checked(self.selected)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.selected = !view.selected;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("compact-button")
                            .label("Compact")
                            .checked(self.compact)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.compact = !view.compact;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("shadow-button")
                            .label("Shadow")
                            .checked(cx.theme().shadow)
                            .on_click(cx.listener(|_, _, window, cx| {
                                let mut theme = cx.theme().clone();
                                theme.shadow = !theme.shadow;
                                cx.set_global::<Theme>(theme);
                                window.refresh();
                            })),
                    ),
            )
            .child(
                section("Dropdown Button").child(
                    DropdownButton::new("btn0")
                        .primary()
                        .button(Button::new("btn").label("Primary Dropdown"))
                        .when(self.compact, |this| this.compact())
                        .loading(self.loading)
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu_with_anchor(Corner::BottomRight, move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check(
                                "Compact",
                                compact,
                                Box::new(ButtonAction::Compact),
                            )
                        }),
                ),
            )
            .child(
                section("Small Size").child(
                    DropdownButton::new("btn-sm")
                        .small()
                        .button(Button::new("btn").label("Small Dropdown"))
                        .when(self.compact, |this| this.compact())
                        .loading(self.loading)
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu(move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check(
                                "Compact",
                                compact,
                                Box::new(ButtonAction::Compact),
                            )
                        }),
                ),
            )
            .child(
                section("Outline").child(
                    DropdownButton::new("btn-outline")
                        .outline()
                        .danger()
                        .button(Button::new("btn").label("Outline Dropdown"))
                        .when(self.compact, |this| this.compact())
                        .loading(self.loading)
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu(move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check(
                                "Compact",
                                compact,
                                Box::new(ButtonAction::Compact),
                            )
                        }),
                ),
            )
            .child(
                section("Ghost").child(
                    DropdownButton::new("btn-ghost")
                        .ghost()
                        .button(Button::new("btn").label("Ghost Dropdown"))
                        .when(self.compact, |this| this.compact())
                        .loading(self.loading)
                        .disabled(self.disabled)
                        .selected(selected)
                        .dropdown_menu(move |this, _, _| {
                            this.menu_with_check(
                                "Disabled",
                                disabled,
                                Box::new(ButtonAction::Disabled),
                            )
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check(
                                "Compact",
                                compact,
                                Box::new(ButtonAction::Compact),
                            )
                        }),
                ),
            )
    }
}
