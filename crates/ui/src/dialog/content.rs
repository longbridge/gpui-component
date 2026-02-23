use gpui::{AnyElement, IntoElement, ParentElement, StyleRefinement, Styled, div};

use crate::StyledExt as _;

/// Content container for a dialog.
pub struct DialogContent {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DialogContent {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for DialogContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for DialogContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for DialogContent {
    type Element = gpui::Div;

    fn into_element(self) -> Self::Element {
        div()
            .w_full()
            .flex_1()
            .px_4()
            .refine_style(&self.style)
            .children(self.children)
    }
}
