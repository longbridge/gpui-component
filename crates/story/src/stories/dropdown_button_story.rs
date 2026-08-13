use gpui::{
    Action, Anchor, App, AppContext as _, Context, Entity, Focusable, InteractiveElement,
    IntoElement, ParentElement as _, Render, Styled as _, Window, prelude::FluentBuilder as _,
};
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};
use gpui_component::{
    ActiveTheme, Disableable, Selectable as _, Sizable as _, Size, Theme,
    button::{Button, ButtonVariants as _, DropdownButton},
    v_flex,
};

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = dropdown_button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
    Shadow,
}

pub struct DropdownButtonStory {
    focus_handle: gpui::FocusHandle,
    disabled: bool,
    loading: bool,
    selected: bool,
    compact: bool,
    size: Size,
}

impl DropdownButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            loading: false,
            selected: false,
            compact: false,
            size: Size::Medium,
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
            .on_action(cx.listener(|this, action: &ChangeStorySize, _, cx| {
                this.size = action.0;
                cx.notify();
            }))
            .on_action(cx.listener(|this, action: &ButtonAction, window, cx| {
                match action {
                    ButtonAction::Disabled => this.disabled = !this.disabled,
                    ButtonAction::Loading => this.loading = !this.loading,
                    ButtonAction::Selected => this.selected = !this.selected,
                    ButtonAction::Compact => this.compact = !this.compact,
                    ButtonAction::Shadow => {
                        let mut theme = cx.theme().clone();
                        theme.shadow = !theme.shadow;
                        cx.set_global::<Theme>(theme);
                        window.refresh();
                    }
                }
                cx.notify();
            }))
            .child(story_toolbar(self.size).dropdown_child(
                Button::new("dropdown-button-options").label("Options"),
                {
                    let shadow = cx.theme().shadow;
                    move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(ButtonAction::Disabled))
                            .menu_with_check("Loading", loading, Box::new(ButtonAction::Loading))
                            .menu_with_check("Selected", selected, Box::new(ButtonAction::Selected))
                            .menu_with_check("Compact", compact, Box::new(ButtonAction::Compact))
                            .menu_with_check("Shadow", shadow, Box::new(ButtonAction::Shadow))
                    }
                },
            ))
            .child(
                section("Default")
                    .description("A primary action with an attached menu.")
                    .child(
                        DropdownButton::new("btn0")
                            .with_size(self.size)
                            .primary()
                            .button(Button::new("btn").label("Primary Dropdown"))
                            .when(self.compact, |this| this.compact())
                            .loading(self.loading)
                            .disabled(self.disabled)
                            .selected(selected)
                            .dropdown_menu_with_anchor(Anchor::BottomRight, move |this, _, _| {
                                this.menu_with_check(
                                    "Disabled",
                                    disabled,
                                    Box::new(ButtonAction::Disabled),
                                )
                                .menu_with_check(
                                    "Loading",
                                    loading,
                                    Box::new(ButtonAction::Loading),
                                )
                                .menu_with_check(
                                    "Selected",
                                    selected,
                                    Box::new(ButtonAction::Selected),
                                )
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
                        .with_size(self.size)
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
                        .with_size(self.size)
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
