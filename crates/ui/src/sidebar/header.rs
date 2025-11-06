use gpui::{
    prelude::FluentBuilder as _, Div, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    Styled,
};

use crate::{h_flex, menu::DropdownMenu, ActiveTheme as _, Collapsible, Selectable};

/// Header for the [`super::Sidebar`]
#[derive(IntoElement)]
pub struct SidebarHeader {
    base: Div,
    selected: bool,
    collapsed: bool,
}

impl SidebarHeader {
    /// Create a new [`SidebarHeader`].
    pub fn new() -> Self {
        Self {
            base: h_flex().gap_2().w_full(),
            selected: false,
            collapsed: false,
        }
    }
}

impl Selectable for SidebarHeader {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Collapsible for SidebarHeader {
    fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl ParentElement for SidebarHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for SidebarHeader {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for SidebarHeader {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for SidebarHeader {}

impl RenderOnce for SidebarHeader {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        h_flex()
            .id("sidebar-header")
            .gap_2()
            .p_2()
            .w_full()
            .justify_between()
            .rounded(cx.theme().radius)
            .hover(|this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .when(self.selected, |this| {
                this.bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
            })
            .child(self.base)
    }
}
