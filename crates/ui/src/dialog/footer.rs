use gpui::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window, div, relative,
};

use crate::{StyledExt as _, h_flex};

/// Footer section of a dialog, typically contains action buttons.
///
/// # Examples
///
/// ```ignore
/// DialogFooter::new()
///     .child(DialogClose::new().child(Button::new("cancel").label("Cancel")))
///     .child(Button::new("confirm").label("Confirm"))
/// ```
#[derive(IntoElement)]
pub struct DialogFooter {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl DialogFooter {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for DialogFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for DialogFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DialogFooter {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .mx_neg_4()
            .mb_neg_4()
            .mt_4()
            .p_4()
            .gap_2()
            .justify_end()
            .line_height(relative(1.))
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, child)| div().id(ix).child(child)),
            )
    }
}
