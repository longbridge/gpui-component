use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonGroup, ButtonVariants},
    h_flex,
    tab::{Tab, TabBar},
    v_flex, IconName, Selectable as _, Sizable, Size,
};

use crate::section;

pub struct TabsStory {
    focus_handle: gpui::FocusHandle,
    active_tab_ix: usize,
    size: Size,
}

impl super::Story for TabsStory {
    fn title() -> &'static str {
        "Tabs"
    }

    fn description() -> &'static str {
        "A set of layered sections of content—known as tab panels—that are displayed one at a time."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl TabsStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab_ix: 0,
            size: Size::default(),
        }
    }

    fn set_active_tab(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        cx.notify();
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl Focusable for TabsStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TabsStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(
                ButtonGroup::new("toggle-size")
                    .child(
                        Button::new("xsmall")
                            .label("XSmall")
                            .selected(self.size == Size::XSmall),
                    )
                    .child(
                        Button::new("small")
                            .label("Small")
                            .selected(self.size == Size::Small),
                    )
                    .child(
                        Button::new("medium")
                            .label("Medium")
                            .selected(self.size == Size::Medium),
                    )
                    .child(
                        Button::new("large")
                            .label("Large")
                            .selected(self.size == Size::Large),
                    )
                    .on_click(cx.listener(|this, selecteds: &Vec<usize>, window, cx| {
                        let size = match selecteds[0] {
                            0 => Size::XSmall,
                            1 => Size::Small,
                            2 => Size::Medium,
                            3 => Size::Large,
                            _ => unreachable!(),
                        };
                        this.set_size(size, window, cx);
                    })),
            )
            .child(
                section("Tabs", cx).child(
                    TabBar::new("tabs")
                        .w_full()
                        .with_size(self.size)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .prefix(
                            h_flex()
                                .mx_1()
                                .child(
                                    Button::new("back")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowLeft),
                                )
                                .child(
                                    Button::new("forward")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::ArrowRight),
                                ),
                        )
                        .child(Tab::new("tab-account", "Account"))
                        .child(Tab::new("tab-profile", "Profile").disabled(true))
                        .child(Tab::new("tab-documents", "Documents"))
                        .child(Tab::new("tab-mail", "Mail"))
                        .child(Tab::new("tab-appearance", "Appearance"))
                        .child(Tab::new("tab-settings", "Settings"))
                        .child(Tab::new("tab-about", "About"))
                        .child(Tab::new("tab-license", "License"))
                        .suffix(
                            h_flex()
                                .mx_1()
                                .child(Button::new("inbox").ghost().xsmall().icon(IconName::Inbox))
                                .child(
                                    Button::new("more")
                                        .ghost()
                                        .xsmall()
                                        .icon(IconName::Ellipsis),
                                ),
                        ),
                ),
            )
            .child(
                section("Pill Tabs", cx).child(
                    TabBar::new("pill-tabs")
                        .w_full()
                        .pill()
                        .with_size(self.size)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-account", "Account"))
                        .child(Tab::new("tab-profile", "Profile").disabled(true))
                        .child(Tab::new("tab-documents", "Documents"))
                        .child(Tab::new("tab-mail", "Mail"))
                        .child(Tab::new("tab-appearance", "Appearance"))
                        .child(Tab::new("tab-settings", "Settings"))
                        .child(Tab::new("tab-about", "About"))
                        .child(Tab::new("tab-license", "License")),
                ),
            )
            .child(
                section("Segmented Tabs", cx).child(
                    TabBar::new("segmented-tabs")
                        .w_full()
                        .segmented()
                        .with_size(self.size)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .children(vec![
                            "Account",
                            "Profile",
                            "Documents",
                            "Mail",
                            "Appearance",
                            "Settings",
                            "About",
                            "License",
                        ]),
                ),
            )
            .child(
                section("Underline Tabs", cx).child(
                    TabBar::new("underline-tabs")
                        .w_full()
                        .px_2()
                        .mx_3()
                        .underline()
                        .with_size(self.size)
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child("Account")
                        .child("Profile")
                        .child("Documents")
                        .child("Mail")
                        .child("Appearance")
                        .child("Settings")
                        .child("About")
                        .child("License"),
                ),
            )
    }
}
