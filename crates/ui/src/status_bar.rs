use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, AnyElement, App, ClickEvent, ElementId, IntoElement,
    ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
};
use smallvec::SmallVec;

use crate::{
    button::{Button, ButtonVariants as _},
    h_flex, ActiveTheme, Icon, Sizable as _, StyledExt,
};

/// A horizontal status bar, usually placed at the bottom of a window or pane.
///
/// It is split into three regions — `left`, `center`, and `right` — that are
/// distributed with `justify_between`. This mirrors the status bars found in
/// native UI frameworks (Windows `StatusStrip`, WPF `StatusBar`, macOS
/// `NSStatusBar`): a container that holds a row of items aligned to either end.
///
/// Each region accepts any element, but [`StatusBarItem`] is provided for the
/// common icon + label + click pattern.
///
/// ```
/// use gpui_component::status_bar::{StatusBar, StatusBarItem};
///
/// let _ = StatusBar::new()
///     .left(StatusBarItem::new("ln").label("Ln 1, Col 1"))
///     .right(StatusBarItem::new("enc").label("UTF-8"));
/// ```
#[derive(IntoElement)]
pub struct StatusBar {
    style: StyleRefinement,
    left: SmallVec<[AnyElement; 1]>,
    center: SmallVec<[AnyElement; 1]>,
    right: SmallVec<[AnyElement; 1]>,
}

impl StatusBar {
    /// Create a new, empty [`StatusBar`].
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            left: SmallVec::new(),
            center: SmallVec::new(),
            right: SmallVec::new(),
        }
    }

    /// Append a child to the left region. Call multiple times to add more.
    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    /// Append a child to the center region. Call multiple times to add more.
    pub fn center(mut self, child: impl IntoElement) -> Self {
        self.center.push(child.into_any_element());
        self
    }

    /// Append a child to the right region. Call multiple times to add more.
    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for StatusBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let region = || h_flex().items_center().gap_3();

        h_flex()
            .w_full()
            // Never let the bar be squeezed to zero height when placed at the
            // bottom of a flex column next to a `flex_1` content area.
            .flex_shrink_0()
            .justify_between()
            .items_center()
            .py_1p5()
            .px_4()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .refine_style(&self.style)
            .child(region().children(self.left))
            .child(region().children(self.center))
            .child(region().children(self.right))
    }
}

/// An item for the [`StatusBar`].
///
/// Renders an optional icon followed by an optional label as a ghost `xsmall`
/// [`Button`], so items share the exact size, hover, and styling of buttons
/// placed in the same status bar. When an `on_click` handler is set the item
/// triggers it on click.
#[derive(IntoElement)]
pub struct StatusBarItem {
    id: ElementId,
    style: StyleRefinement,
    icon: Option<Icon>,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl StatusBarItem {
    /// Create a new [`StatusBarItem`] with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            icon: None,
            label: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// Set the leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the label text.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the tooltip text shown on hover.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set the click handler.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }
}

impl Styled for StatusBarItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StatusBarItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        // Render as a ghost `xsmall` Button so items share the exact size,
        // hover, and styling of buttons placed in the same status bar.
        Button::new(self.id)
            .ghost()
            .xsmall()
            .when_some(self.icon, |this, icon| this.icon(icon))
            .when_some(self.label, |this, label| this.label(label))
            .when_some(self.tooltip, |this, tooltip| this.tooltip(tooltip))
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .refine_style(&self.style)
    }
}
