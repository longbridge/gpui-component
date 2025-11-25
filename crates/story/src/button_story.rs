use gpui::{
    Action, App, AppContext as _, Axis, ClickEvent, Context, Entity, Focusable, InteractiveElement,
    IntoElement, ParentElement as _, Render, Styled as _, Window, prelude::FluentBuilder, px,
};

use gpui_component::{
    ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable as _, Theme,
    button::{Button, ButtonCustomVariant, ButtonGroup, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex, v_flex,
};
use serde::Deserialize;

use crate::section;

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = button_story, no_json)]
enum ButtonAction {
    Disabled,
    Loading,
    Selected,
    Compact,
}

pub struct ButtonStory {
    focus_handle: gpui::FocusHandle,
    disabled: bool,
    loading: bool,
    selected: bool,
    compact: bool,
    toggle_multiple: bool,
}

impl ButtonStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            disabled: false,
            loading: false,
            selected: false,
            compact: false,
            toggle_multiple: false,
        })
    }

    fn on_click(ev: &ClickEvent, _: &mut Window, _: &mut App) {
        println!("Button clicked {:?}", ev);
    }

    fn on_hover(hovered: &bool, _: &mut Window, _: &mut App) {
        println!("Button hovered {:?}", hovered);
    }
}

impl super::Story for ButtonStory {
    fn title() -> &'static str {
        "Button"
    }

    fn description() -> &'static str {
        "Displays a button or a component that looks like a button."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for ButtonStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ButtonStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.disabled;
        let loading = self.loading;
        let selected = self.selected;
        let compact = self.compact;
        let toggle_multiple = self.toggle_multiple;

        let custom_variant = ButtonCustomVariant::new(cx)
            .color(cx.theme().magenta)
            .foreground(cx.theme().primary_foreground)
            .border(cx.theme().magenta)
            .hover(cx.theme().magenta.opacity(0.1))
            .active(cx.theme().magenta);

        v_flex()
            .on_action(
                cx.listener(|this, action: &ButtonAction, _, _| match action {
                    ButtonAction::Disabled => this.disabled = !this.disabled,
                    ButtonAction::Loading => this.loading = !this.loading,
                    ButtonAction::Selected => this.selected = !this.selected,
                    ButtonAction::Compact => this.compact = !this.compact,
                }),
            )
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
                section("Normal Button")
                    .max_w_lg()
                    .child(
                        Button::new("button-1")
                            .primary()
                            .label("Primary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-2")
                            .label("Secondary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-4")
                            .danger()
                            .label("Danger Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-4-warning")
                            .warning()
                            .label("Warning Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-4-success")
                            .success()
                            .label("Success Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-5-info")
                            .info()
                            .label("Info Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-5-ghost")
                            .ghost()
                            .label("Ghost Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-5-link")
                            .link()
                            .label("Link Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    )
                    .child(
                        Button::new("button-5-text")
                            .text()
                            .label("Text Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click)
                            .on_hover(Self::on_hover),
                    ),
            )
            .child(
                section("Button with Icon")
                    .child(
                        Button::new("button-icon-1")
                            .primary()
                            .label("Confirm")
                            .icon(IconName::Check)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-icon-2")
                            .label("Abort")
                            .icon(IconName::Close)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-icon-3")
                            .label("Maximize")
                            .icon(Icon::new(IconName::Maximize))
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-icon-4")
                            .primary()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child("Custom Child")
                                    .child(IconName::ChevronDown)
                                    .child(IconName::Eye),
                            )
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-icon-5-ghost")
                            .ghost()
                            .icon(IconName::Check)
                            .label("Confirm")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-icon-6-link")
                            .link()
                            .icon(IconName::Check)
                            .label("Link")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-icon-6-text")
                            .text()
                            .icon(IconName::Check)
                            .label("Text Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("Outline Button")
                    .max_w_lg()
                    .child(
                        Button::new("button-outline-1")
                            .primary()
                            .outline()
                            .label("Primary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-2")
                            .outline()
                            .label("Secondary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-4-danger")
                            .danger()
                            .outline()
                            .label("Danger Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-4-warning")
                            .warning()
                            .outline()
                            .label("Warning Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-4-success")
                            .success()
                            .outline()
                            .label("Success Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-info")
                            .info()
                            .outline()
                            .label("Info Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-ghost")
                            .ghost()
                            .outline()
                            .label("Ghost Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-link")
                            .link()
                            .outline()
                            .label("Link Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-text")
                            .text()
                            .outline()
                            .label("Text Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("With Dropdown Caret")
                    .max_w_lg()
                    .child(
                        Button::new("button-outline-1")
                            .primary()
                            .dropdown_caret(true)
                            .label("Primary Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-2")
                            .outline()
                            .label("Secondary Button")
                            .dropdown_caret(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-ghost")
                            .ghost()
                            .dropdown_caret(true)
                            .label("Ghost Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-link")
                            .link()
                            .dropdown_caret(true)
                            .label("Link Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-5-text")
                            .outline()
                            .small()
                            .dropdown_caret(true)
                            .label("Small Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("Small Size")
                    .child(
                        Button::new("button-6")
                            .label("Primary Button")
                            .icon(IconName::Check)
                            .primary()
                            .small()
                            .loading(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-7")
                            .label("Secondary Button")
                            .small()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-8")
                            .label("Danger Button")
                            .danger()
                            .small()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-8-outline")
                            .label("Outline Button")
                            .outline()
                            .small()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-8-ghost")
                            .label("Ghost Button")
                            .ghost()
                            .small()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-8-link")
                            .label("Link Button")
                            .link()
                            .small()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("XSmall Size")
                    .child(
                        Button::new("button-xs-1")
                            .label("Primary Button")
                            .primary()
                            .icon(IconName::Check)
                            .xsmall()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-xs-2")
                            .label("Secondary Button")
                            .xsmall()
                            .loading(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-xs-3")
                            .label("Danger Button")
                            .danger()
                            .xsmall()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-xs-3-ghost")
                            .label("Ghost Button")
                            .ghost()
                            .xsmall()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-xs-3-outline")
                            .label("Outline Button")
                            .outline()
                            .xsmall()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-xs-3-link")
                            .label("Link Button")
                            .link()
                            .xsmall()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
            .child(
                section("Button Group").child(
                    ButtonGroup::new("button-group")
                        .outline()
                        .disabled(disabled)
                        .child(
                            Button::new("button-one")
                                .label("One")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            Button::new("button-two")
                                .label("Two")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            Button::new("button-three")
                                .label("Three")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        ),
                ),
            )
            .child(
                section("Button Group (Vertical)").child(
                    ButtonGroup::new("button-group-vertical")
                        .outline()
                        .layout(Axis::Vertical)
                        .disabled(disabled)
                        .child(
                            Button::new("button-one")
                                .label("One")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            Button::new("button-two")
                                .label("Two")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        )
                        .child(
                            Button::new("button-three")
                                .label("Three")
                                .disabled(disabled)
                                .selected(selected)
                                .when(compact, |this| this.compact())
                                .on_click(Self::on_click),
                        ),
                ),
            )
            .child(
                section("Toggle Button Group")
                    .sub_title(
                        Checkbox::new("multiple-button")
                            .text_sm()
                            .label("Multiple")
                            .checked(toggle_multiple)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.toggle_multiple = !view.toggle_multiple;
                                cx.notify();
                            })),
                    )
                    .child(
                        ButtonGroup::new("toggle-button-group")
                            .outline()
                            .compact()
                            .multiple(toggle_multiple)
                            .child(
                                Button::new("disabled-toggle-button")
                                    .label("Disabled")
                                    .selected(disabled),
                            )
                            .child(
                                Button::new("loading-toggle-button")
                                    .label("Loading")
                                    .selected(loading),
                            )
                            .child(
                                Button::new("selected-toggle-button")
                                    .label("Selected")
                                    .selected(selected),
                            )
                            .child(
                                Button::new("compact-toggle-button")
                                    .label("Compact")
                                    .selected(compact),
                            )
                            .on_click(cx.listener(|view, selected: &Vec<usize>, _, cx| {
                                view.disabled = selected.contains(&0);
                                view.loading = selected.contains(&1);
                                view.selected = selected.contains(&2);
                                view.compact = selected.contains(&3);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("Icon Button")
                    .child(
                        Button::new("icon-button-primary")
                            .icon(IconName::Search)
                            .loading_icon(IconName::LoaderCircle)
                            .primary()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-secondary")
                            .icon(IconName::Info)
                            .loading(true)
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-danger")
                            .icon(IconName::Close)
                            .danger()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-small-primary")
                            .icon(IconName::Search)
                            .small()
                            .primary()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-outline")
                            .icon(IconName::Search)
                            .outline()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-ghost")
                            .icon(IconName::ArrowLeft)
                            .loading_icon(IconName::LoaderCircle)
                            .ghost()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    ),
            )
            .child(
                section("Icon Button")
                    .child(
                        Button::new("icon-button-4")
                            .icon(IconName::Info)
                            .small()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-5")
                            .icon(IconName::Close)
                            .small()
                            .danger()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-6")
                            .icon(IconName::Search)
                            .small()
                            .primary()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-7")
                            .icon(IconName::Info)
                            .xsmall()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-8")
                            .icon(IconName::Close)
                            .xsmall()
                            .danger()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    )
                    .child(
                        Button::new("icon-button-9")
                            .icon(IconName::Heart)
                            .size(px(24.))
                            .ghost()
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact()),
                    ),
            )
            .child(
                section("Custom Button")
                    .child(
                        Button::new("button-6-custom")
                            .custom(custom_variant)
                            .label("Custom Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-6-custom")
                            .outline()
                            .custom(custom_variant)
                            .label("Outline Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    )
                    .child(
                        Button::new("button-outline-6-custom-1")
                            .outline()
                            .icon(IconName::Bell)
                            .custom(custom_variant)
                            .label("Icon Button")
                            .disabled(disabled)
                            .selected(selected)
                            .loading(loading)
                            .when(compact, |this| this.compact())
                            .on_click(Self::on_click),
                    ),
            )
    }
}
