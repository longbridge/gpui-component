use gpui::{AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

enum Child {
    Element(AnyElement),
    Content(AnyElement),
}

/// An unstyled controlled region whose content can be expanded or collapsed.
#[derive(IntoElement)]
pub struct Collapsible {
    base: Div,
    children: Vec<Child>,
    open: bool,
}

impl Collapsible {
    pub fn new() -> Self {
        Self {
            base: div(),
            children: Vec::new(),
            open: false,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.children
            .push(Child::Content(content.into_any_element()));
        self
    }
}

impl Default for Collapsible {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Collapsible {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(Child::Element));
    }
}

impl RenderOnce for Collapsible {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .children(self.children.into_iter().filter_map(|child| match child {
                Child::Element(element) => Some(element),
                Child::Content(content) => self.open.then_some(content),
            }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, InteractiveElement as _, Render, TestAppContext, px};

    use super::*;

    struct Harness(bool);

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Collapsible::new()
                .open(self.0)
                .child(div().debug_selector(|| "trigger".into()).size(px(10.)))
                .content(div().debug_selector(|| "content".into()).size(px(10.)))
        }
    }

    #[gpui::test]
    fn content_is_only_rendered_while_open(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Harness(false));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("trigger").is_some());
        assert!(cx.debug_bounds("content").is_none());

        let (_, cx) = cx.add_window_view(|_, _| Harness(true));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("trigger").is_some());
        assert!(cx.debug_bounds("content").is_some());
    }
}
