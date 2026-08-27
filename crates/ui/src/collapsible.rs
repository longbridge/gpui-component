use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
};

use crate::StyledExt;

/// An interactive element which expands/collapses.
#[derive(IntoElement)]
pub struct Collapsible {
    base: gpui_base::Collapsible,
    style: StyleRefinement,
}

impl Collapsible {
    /// Creates a new `Collapsible` instance.
    pub fn new() -> Self {
        Self {
            base: gpui_base::Collapsible::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Sets whether the collapsible is open. default is false.
    pub fn open(mut self, open: bool) -> Self {
        self.base = self.base.open(open);
        self
    }

    /// Sets the content of the collapsible.
    ///
    /// If `open` is false, content will be hidden.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.base = self.base.content(content);
        self
    }
}

impl Styled for Collapsible {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Collapsible {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Collapsible {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base.v_flex().refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, InteractiveElement as _, Render, TestAppContext, div, px};

    use super::*;

    struct Harness(bool);

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Collapsible::new()
                .open(self.0)
                .child(
                    div()
                        .debug_selector(|| "collapsible-trigger".into())
                        .size(px(10.)),
                )
                .content(
                    div()
                        .debug_selector(|| "collapsible-content".into())
                        .size(px(10.)),
                )
        }
    }

    #[gpui::test]
    fn facade_preserves_vertical_layout_and_visibility(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Harness(true));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let trigger = cx.debug_bounds("collapsible-trigger").unwrap();
        let content = cx.debug_bounds("collapsible-content").unwrap();
        assert!(trigger.origin.y < content.origin.y);

        let (_, cx) = cx.add_window_view(|_, _| Harness(false));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("collapsible-trigger").is_some());
        assert!(cx.debug_bounds("collapsible-content").is_none());
    }
}
