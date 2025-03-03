use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window,
};

use gpui_component::{
    tab::{Tab, TabBar},
    v_flex,
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
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-1", "Account"))
                        .child(Tab::new("tab-2", "Profile"))
                        .child(Tab::new("tab-3", "Settings")),
                ),
            )
            .child(
                section("Pills Tabs", cx).child(
                    TabBar::new("pills-tabs")
                        .pill()
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-1", "Account"))
                        .child(Tab::new("tab-2", "Profile"))
                        .child(Tab::new("tab-3", "Settings")),
                ),
            )
            .child(
                section("Underline Tabs", cx).child(
                    TabBar::new("underline-tabs")
                        .underline()
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, window, cx| {
                            this.set_active_tab(*ix, window, cx);
                        }))
                        .child(Tab::new("tab-1", "Account"))
                        .child(Tab::new("tab-2", "Profile"))
                        .child(Tab::new("tab-3", "Settings")),
                ),
            )
    }
}
