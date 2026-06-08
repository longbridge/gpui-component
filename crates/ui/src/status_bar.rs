use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _,
};
use smallvec::SmallVec;

use crate::{
    ActiveTheme, Sizable as _, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    separator::Separator,
};

/// A horizontal status bar, usually placed at the bottom of a window or pane.
///
/// It is split into three regions — `left`, `center`, and `right`. This mirrors
/// the status bars found in native UI frameworks (Windows `StatusStrip`, WPF
/// `StatusBar`, macOS `NSStatusBar`): a container that holds a row of items
/// aligned to either end.
///
/// Each region accepts any [`IntoElement`], so a string, an [`Icon`](crate::Icon),
/// a [`Button`], a custom layout, etc. can be passed directly. Use a plain
/// string for a non-interactive label, [`StatusBar::button`] for a
/// consistently-sized clickable button, and [`StatusBar::separator`] for a
/// vertical separator between items.
///
/// `left` and `right` pin items to each end. `child`/`children` add to the
/// center region, whose alignment follows the pinned ends: centered with both
/// `left` and `right`, end-aligned with only `left`, and start-aligned
/// otherwise (only `right`, or neither — like a plain container).
///
/// ```
/// use gpui_component::status_bar::StatusBar;
///
/// let _ = StatusBar::new()
///     .left("Ln 1, Col 1")
///     .right(StatusBar::button("encoding").label("UTF-8"));
/// ```
#[derive(IntoElement)]
pub struct StatusBar {
    style: StyleRefinement,
    left: SmallVec<[AnyElement; 1]>,
    right: SmallVec<[AnyElement; 1]>,
    children: SmallVec<[AnyElement; 1]>,
}

impl StatusBar {
    /// Create a new, empty [`StatusBar`].
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            left: SmallVec::new(),
            right: SmallVec::new(),
            children: SmallVec::new(),
        }
    }

    /// A ghost, xsmall [`Button`] preset for a status bar, so every status bar
    /// button shares a consistent size. Chain `label`, `icon`, `on_click`, etc.
    pub fn button(id: impl Into<ElementId>) -> Button {
        Button::new(id).ghost().xsmall()
    }

    /// A vertical separator for splitting status bar items into groups.
    pub fn separator() -> Separator {
        Separator::vertical().h_3()
    }

    /// Append an element to the left region. Call multiple times to add more.
    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    /// Append an element to the right region. Call multiple times to add more.
    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

/// `child` / `children` add to the center region, so a `StatusBar` without
/// `left`/`right` items behaves like a plain container.
impl ParentElement for StatusBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for StatusBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // The center aligns by which ends are pinned: centered with both left
        // and right, end-aligned with only left, otherwise start-aligned (only
        // right, or neither) — so a bar with just `child`s reads like a container.
        let has_left = !self.left.is_empty();
        let has_right = !self.right.is_empty();
        let region = || h_flex().items_center().gap_1();

        h_flex()
            .items_center()
            .gap_1()
            .py_1()
            .px_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .when(has_left, |this| this.child(region().children(self.left)))
            // The center region is always present as a flex spacer, so `left`
            // and `right` are pushed to each end even without center content.
            .child(
                region()
                    .flex_1()
                    .when(has_left && has_right, |this| this.justify_center())
                    .when(has_left && !has_right, |this| this.justify_end())
                    .children(self.children),
            )
            .when(has_right, |this| this.child(region().children(self.right)))
    }
}
