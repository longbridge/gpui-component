use gpui::{
    div, AnyElement, ClickEvent, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
};

use crate::{ActiveTheme as _, StyledExt};

/// A Link element like a `<a>` tag in HTML.
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    style: StyleRefinement,
    href: Option<SharedString>,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>>,
    children: Vec<AnyElement>,
}

impl Link {
    /// Create a new Link element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            href: None,
            on_click: None,
            disabled: false,
            children: Vec::new(),
        }
    }

    /// Set the href of the link.
    pub fn href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Set the click handler of the link.
    ///
    /// If this set, the handler will be called when the link is clicked.
    /// Otherwise, the link will only open the href if set.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Set the disabled state, default false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Link {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Link {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Link {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let href = self.href.clone();
        let on_click = self.on_click;

        div()
            .id(self.id)
            .text_color(cx.theme().link)
            .text_decoration_1()
            .text_decoration_color(cx.theme().link)
            .hover(|this| {
                this.text_color(cx.theme().link.opacity(0.8))
                    .text_decoration_1()
            })
            .active(|this| {
                this.text_color(cx.theme().link.opacity(0.6))
                    .text_decoration_1()
            })
            .cursor_pointer()
            .refine_style(&self.style)
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_click({
                move |e, window, cx| {
                    if let Some(href) = &href {
                        cx.open_url(&href.clone());
                    }
                    if let Some(on_click) = &on_click {
                        on_click(e, window, cx);
                    }
                }
            })
            .children(self.children)
    }
}
