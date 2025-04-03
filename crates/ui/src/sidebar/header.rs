use gpui::{
    prelude::FluentBuilder as _, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled,
};

use crate::{h_flex, popup_menu::PopupMenuExt, ActiveTheme as _, Collapsible, Selectable};

#[derive(IntoElement)]
pub struct SidebarHeader {
    id: ElementId,
    base: Div,
    selected: bool,
    collapsed: bool,
}

impl SidebarHeader {
    pub fn new() -> Self {
        Self {
            id: SharedString::from("sidebar-header").into(),
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

    fn element_id(&self) -> &gpui::ElementId {
        &self.id
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
impl PopupMenuExt for SidebarHeader {}
impl RenderOnce for SidebarHeader {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        h_flex()
            .id(self.id)
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
