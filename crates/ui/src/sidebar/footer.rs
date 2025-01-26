use gpui::{
    prelude::FluentBuilder as _, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled,
};

use crate::{h_flex, popup_menu::PopupMenuExt, ActiveTheme as _, Collapsible, Selectable};

#[derive(IntoElement)]
pub struct SidebarFooter {
    id: ElementId,
    base: Div,
    selected: bool,
    is_collapsed: bool,
}

impl SidebarFooter {
    pub fn new() -> Self {
        Self {
            id: SharedString::from("sidebar-footer").into(),
            base: h_flex().gap_2().w_full(),
            selected: false,
            is_collapsed: false,
        }
    }
}
impl Selectable for SidebarFooter {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn element_id(&self) -> &gpui::ElementId {
        &self.id
    }
}
impl Collapsible for SidebarFooter {
    fn is_collapsed(&self) -> bool {
        self.is_collapsed
    }

    fn collapsed(mut self, collapsed: bool) -> Self {
        self.is_collapsed = collapsed;
        self
    }
}
impl ParentElement for SidebarFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.base.extend(elements);
    }
}
impl Styled for SidebarFooter {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}
impl PopupMenuExt for SidebarFooter {}
impl RenderOnce for SidebarFooter {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        h_flex()
            .id(self.id)
            .gap_2()
            .p_2()
            .w_full()
            .justify_between()
            .cursor_pointer()
            .rounded_md()
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
