use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    tab::{Tab, TabBar},
    v_flex, IconName, Sizable,
};

use crate::section;

pub struct TabsStory {
    focus_handle: gpui::FocusHandle,
    active_tab_ix: usize,
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
        }
    }

    fn set_active_tab(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
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
                section("Normal Tabs", cx).child(
                    TabBar::new("normal-tabs")
                        .w_full()
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
                section("Pills Tabs", cx).child(
                    TabBar::new("pills-tabs")
                        .w_full()
                        .pill()
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-account", "Account"))
                        .child(Tab::new("tab-profile", "Profile").disabled(true))
                        .child(Tab::new("tab-documents", "Documents"))
                        .child(Tab::new("tab-mail", "Mail"))
                        .child(Tab::new("tab-appearance", "Appearance"))
                        .child(Tab::new("tab-settings", "Settings")),
                ),
            )
            .child(
                section("Segmented Tabs", cx).child(
                    TabBar::new("segmented-tabs")
                        .w_full()
                        .segmented()
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-account", "Account"))
                        .child(Tab::new("tab-profile", "Profile").disabled(true))
                        .child(Tab::new("tab-documents", "Documents"))
                        .child(Tab::new("tab-mail", "Mail"))
                        .child(Tab::new("tab-appearance", "Appearance"))
                        .child(Tab::new("tab-settings", "Settings")),
                ),
            )
            .child(
                section("Underline Tabs", cx).child(
                    TabBar::new("underline-tabs")
                        .w_full()
                        .underline()
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-account", "Account"))
                        .child(Tab::new("tab-profile", "Profile").disabled(true))
                        .child(Tab::new("tab-documents", "Documents"))
                        .child(Tab::new("tab-mail", "Mail"))
                        .child(Tab::new("tab-appearance", "Appearance"))
                        .child(Tab::new("tab-settings", "Settings")),
                ),
            )
    }
}
