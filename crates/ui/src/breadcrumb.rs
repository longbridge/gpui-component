use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder as _, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, WindowContext,
};

use crate::{h_flex, theme::ActiveTheme, Icon, IconName};

#[derive(IntoElement)]
pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
}

#[derive(IntoElement)]
pub struct BreadcrumbItem {
    id: ElementId,
    text: SharedString,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut WindowContext)>>,
    disabled: bool,
    is_last: bool,
}

impl BreadcrumbItem {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            on_click: None,
            disabled: false,
            is_last: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// For internal use only.
    fn is_last(mut self, is_last: bool) -> Self {
        self.is_last = is_last;
        self
    }
}

impl RenderOnce for BreadcrumbItem {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        div()
            .id(self.id)
            .child(self.text)
            .text_color(cx.theme().muted_foreground)
            .when(self.is_last, |this| this.text_color(cx.theme().foreground))
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .when(!self.disabled, |this| {
                this.when_some(self.on_click, |this, on_click| {
                    this.cursor_pointer().on_click(move |event, cx| {
                        on_click(event, cx);
                    })
                })
            })
    }
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add an item to the breadcrumb.
    pub fn item(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self
    }
}

#[derive(IntoElement)]
struct BreadcrumbSeparator;
impl RenderOnce for BreadcrumbSeparator {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        Icon::new(IconName::ChevronRight)
            .text_color(cx.theme().muted_foreground)
            .size_3p5()
            .into_any_element()
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let items_count = self.items.len();

        let mut children = vec![];
        for (ix, item) in self.items.into_iter().enumerate() {
            let is_last = ix == items_count - 1;

            children.push(item.is_last(is_last).into_any_element());
            if !is_last {
                children.push(BreadcrumbSeparator.into_any_element());
            }
        }

        h_flex()
            .gap_1p5()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .children(children)
    }
}
