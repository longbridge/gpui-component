use crate::{ActiveTheme, Selectable};
use gpui::prelude::FluentBuilder as _;
use gpui::{Window, AppContext, 
    div, px, AnyElement, Div, ElementId, InteractiveElement, IntoElement, ParentElement as _,
    RenderOnce, Stateful, StatefulInteractiveElement, Styled, 
};

#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    base: Stateful<Div>,
    label: AnyElement,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    disabled: bool,
    selected: bool,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>, label: impl IntoElement) -> Self {
        let id: ElementId = id.into();
        Self {
            id: id.clone(),
            base: div().id(id).gap_1().py_1p5().px_3().h(px(30.)),
            label: label.into_any_element(),
            disabled: false,
            selected: false,
            prefix: None,
            suffix: None,
        }
    }

    /// Set the left side of the tab
    pub fn prefix(mut self, prefix: impl Into<AnyElement>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set the right side of the tab
    pub fn suffix(mut self, suffix: impl Into<AnyElement>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Set disabled state to the tab
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Tab {
    fn element_id(&self) -> &ElementId {
        &self.id
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Styled for Tab {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Tab {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (text_color, bg_color) = match (self.selected, self.disabled) {
            (true, false) => (cx.theme().tab_active_foreground, cx.theme().tab_active),
            (false, false) => (cx.theme().muted_foreground, cx.theme().tab),
            // disabled
            (true, true) => (cx.theme().muted_foreground, cx.theme().tab_active),
            (false, true) => (cx.theme().muted_foreground, cx.theme().tab),
        };

        self.base
            .flex()
            .items_center()
            .flex_shrink_0()
            .cursor_pointer()
            .overflow_hidden()
            .text_color(text_color)
            .bg(bg_color)
            .border_x_1()
            .border_color(cx.theme().transparent)
            .when(self.selected, |this| this.border_color(cx.theme().border))
            .text_sm()
            .when_some(self.prefix, |this, prefix| {
                this.child(prefix).text_color(text_color)
            })
            .child(div().text_ellipsis().child(self.label))
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}
