use gpui::{
    Action, Anchor, App, AppContext as _, Context, Entity, Focusable, InteractiveElement,
    IntoElement, ParentElement as _, Render, SharedString, Styled as _, Window,
    prelude::FluentBuilder as _,
};
use serde::Deserialize;

use crate::{ChangeStorySize, section, story_toolbar};
use gpui_component::{
    ActiveTheme, Disableable, Selectable as _, Sizable as _, Size, Theme,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex, v_flex,
};

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = dropdown_button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
    Shadow,
    ExportCsv,
    ExportPdf,
    SaveCopy,
    OpenRecent,
    ChooseColumns,
}

pub struct DropdownButtonStory {
    focus_handle: gpui::FocusHandle,
    disabled: bool,
    loading: bool,
    selected: bool,
    compact: bool,
    size: Size,
    last_action: SharedString,
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
            last_action: "Nothing yet".into(),
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
        let view = cx.entity();

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
                    ButtonAction::ExportCsv => this.last_action = "Exported as CSV".into(),
                    ButtonAction::ExportPdf => this.last_action = "Exported as PDF".into(),
                    ButtonAction::SaveCopy => this.last_action = "Saved a copy".into(),
                    ButtonAction::OpenRecent => this.last_action = "Opened recent files".into(),
                    ButtonAction::ChooseColumns => {
                        this.last_action = "Opened column chooser".into()
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
                h_flex()
                    .gap_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Last action:")
                    .child(self.last_action.clone()),
            )
            .child(
                section("Basic split")
                    .description("Run the default export or choose another format.")
                    .child(
                        DropdownButton::new("export")
                            .with_size(self.size)
                            .primary()
                            .button(
                                Button::new("export-default")
                                    .label("Export")
                                    .when(self.compact, |this| this.compact())
                                    .on_click({
                                        let view = view.clone();
                                        move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.last_action = "Exported with defaults".into();
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .disabled(self.disabled)
                            .selected(selected)
                            .dropdown_menu_with_anchor(Anchor::TopRight, move |this, _, _| {
                                this.menu("Export as CSV", Box::new(ButtonAction::ExportCsv))
                                    .menu("Export as PDF", Box::new(ButtonAction::ExportPdf))
                            }),
                    ),
            )
            .child(
                section("Inner button options")
                    .description(
                        "Loading, compact, tooltip and click behavior belong to the action.",
                    )
                    .child(
                        DropdownButton::new("save")
                            .with_size(self.size)
                            .outline()
                            .button(
                                Button::new("save-default")
                                    .label("Save")
                                    .tooltip("Save the current document")
                                    .when(compact, |this| this.compact())
                                    .loading(loading)
                                    .on_click({
                                        let view = view.clone();
                                        move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.last_action = "Saved document".into();
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .disabled(disabled)
                            .dropdown_menu(move |this, _, _| {
                                this.menu("Save a copy…", Box::new(ButtonAction::SaveCopy))
                            }),
                    ),
            )
            .child(
                section("Inherited styling")
                    .description(
                        "With no outer variant or size, both halves follow the inner button.",
                    )
                    .child(
                        DropdownButton::new("recent")
                            .button(
                                Button::new("recent-default")
                                    .label("Recent")
                                    .ghost()
                                    .small()
                                    .on_click({
                                        let view = view.clone();
                                        move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                this.last_action = "Opened latest file".into();
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .selected(selected)
                            .disabled(disabled)
                            .dropdown_menu(move |this, _, _| {
                                this.menu(
                                    "Browse recent files…",
                                    Box::new(ButtonAction::OpenRecent),
                                )
                            }),
                    ),
            )
            .child(
                section("Menu only")
                    .description(
                        "The menu trigger still renders when no default action is provided.",
                    )
                    .child(
                        DropdownButton::new("columns")
                            .with_size(self.size)
                            .secondary()
                            .disabled(disabled)
                            .dropdown_menu(move |this, _, _| {
                                this.menu("Choose columns…", Box::new(ButtonAction::ChooseColumns))
                            }),
                    ),
            )
    }
}
