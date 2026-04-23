use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Corners, Edges, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use smallvec::{SmallVec, smallvec};

use crate::{
    ActiveTheme, Disableable, Icon, Sizable, Size, StyledExt, h_flex, tooltip::ComponentTooltip,
};

#[derive(Default, Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ToggleVariant {
    #[default]
    Ghost,
    Outline,
}

pub trait ToggleVariants: Sized {
    /// Set the variant of the toggle.
    fn with_variant(self, variant: ToggleVariant) -> Self;
    /// Set the variant to ghost.
    fn ghost(self) -> Self {
        self.with_variant(ToggleVariant::Ghost)
    }
    /// Set the variant to outline.
    fn outline(self) -> Self {
        self.with_variant(ToggleVariant::Outline)
    }
}

#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    border_corners: Corners<bool>,
    border_edges: Edges<bool>,
    children: SmallVec<[AnyElement; 1]>,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: ComponentTooltip,
}

impl Toggle {
    /// Create a new Toggle element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            checked: false,
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            border_corners: Corners { top_left: true, top_right: true, bottom_left: true, bottom_right: true },
            border_edges: Edges::all(true),
            children: smallvec![],
            on_click: None,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Set tooltip text for the toggle.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Add a label to the toggle.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        self.children.push(label.into_any_element());
        self
    }

    /// Add icon to the toggle.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        let icon: Icon = icon.into();
        self.children.push(icon.into());
        self
    }

    /// Set the checked state of the toggle, default: false
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set the callback to be called when the toggle is clicked.
    ///
    /// The `&bool` parameter represents the new checked state of the toggle.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub(crate) fn border_corners(mut self, corners: impl Into<Corners<bool>>) -> Self {
        self.border_corners = corners.into();
        self
    }

    pub(crate) fn border_edges(mut self, edges: impl Into<Edges<bool>>) -> Self {
        self.border_edges = edges.into();
        self
    }
}

impl ToggleVariants for Toggle {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl ParentElement for Toggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Toggle {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Toggle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Toggle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let hoverable = !disabled && !checked;
        let rounding = cx.theme().radius;

        div()
            .id(self.id)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .map(|this| match self.size {
                Size::XSmall => this.min_w_5().h_5().px_0p5().text_xs(),
                Size::Small => this.min_w_6().h_6().px_1().text_sm(),
                Size::Large => this.min_w_9().h_9().px_3().text_lg(),
                _ => this.min_w_8().h_8().px_2(),
            })
            .when(self.border_corners.top_left, |this| {
                this.rounded_tl(rounding)
            })
            .when(self.border_corners.top_right, |this| {
                this.rounded_tr(rounding)
            })
            .when(self.border_corners.bottom_left, |this| {
                this.rounded_bl(rounding)
            })
            .when(self.border_corners.bottom_right, |this| {
                this.rounded_br(rounding)
            })
            .when(self.variant == ToggleVariant::Outline, |this| {
                this.when(self.border_edges.left, |this| this.border_l_1())
                    .when(self.border_edges.right, |this| this.border_r_1())
                    .when(self.border_edges.top, |this| this.border_t_1())
                    .when(self.border_edges.bottom, |this| this.border_b_1())
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .when(cx.theme().shadow, |this| this.shadow_xs())
            })
            .when(hoverable, |this| {
                this.hover(|this| {
                    this.bg(cx.theme().accent)
                        .text_color(cx.theme().accent_foreground)
                })
            })
            .when(checked, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .refine_style(&self.style)
            .children(self.children)
            .when(!disabled, |this| {
                this.when_some(self.on_click, |this, on_click| {
                    this.on_click(move |_, window, cx| on_click(&!checked, window, cx))
                })
            })
            .map(|this| self.tooltip.apply(this))
    }
}

/// A group of toggles.
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    segmented: bool,
    items: Vec<Toggle>,
    on_click: Option<Rc<dyn Fn(&Vec<bool>, &mut Window, &mut App) + 'static>>,
}

impl ToggleGroup {
    /// Create a new ToggleGroup element.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            segmented: false,
            items: Vec::new(),
            on_click: None,
        }
    }

    /// Add a child [`Toggle`] to the group.
    pub fn child(mut self, toggle: impl Into<Toggle>) -> Self {
        self.items.push(toggle.into());
        self
    }

    /// Add multiple [`Toggle`]s to the group.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Toggle>>) -> Self {
        self.items.extend(children.into_iter().map(Into::into));
        self
    }

    /// Set the callback to be called when the toggle group changes.
    ///
    /// The `&Vec<bool>` parameter represents the new check state of each [`Toggle`] in the group.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&Vec<bool>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Render the group as a connected segmented control.
    ///
    /// This keeps the existing multi-toggle behavior, but removes the default
    /// gap and joins adjacent item borders into a single segmented outline.
    pub fn segmented(mut self) -> Self {
        self.segmented = true;
        self
    }
}

impl Sizable for ToggleGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ToggleVariants for ToggleGroup {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Disableable for ToggleGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for ToggleGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let items_len = self.items.len();
        let checks = self
            .items
            .iter()
            .map(|item| item.checked)
            .collect::<Vec<bool>>();
        let state = Rc::new(Cell::new(None));

        h_flex()
            .id(self.id)
            .when(!self.segmented, |this| this.gap_2())
            .refine_style(&self.style)
            .children(self.items.into_iter().enumerate().map({
                |(ix, item)| {
                    let state = state.clone();
                    let item = if !self.segmented || items_len == 1 {
                        item
                    } else if ix == 0 {
                        item.border_corners(Corners {
                            top_left: true,
                            top_right: false,
                            bottom_left: true,
                            bottom_right: false,
                        })
                        .border_edges(Edges {
                            left: true,
                            top: true,
                            right: true,
                            bottom: true,
                        })
                    } else if ix == items_len - 1 {
                        item.border_corners(Corners {
                            top_left: false,
                            top_right: true,
                            bottom_left: false,
                            bottom_right: true,
                        })
                        .border_edges(Edges {
                            left: false,
                            top: true,
                            right: true,
                            bottom: true,
                        })
                    } else {
                        item.border_corners(Corners { top_left: false, top_right: false, bottom_left: false, bottom_right: false })
                            .border_edges(Edges {
                                left: false,
                                top: true,
                                right: true,
                                bottom: true,
                            })
                    };

                    item.disabled(disabled)
                        .with_size(self.size)
                        .with_variant(self.variant)
                        .on_click(move |_, _, _| {
                            state.set(Some(ix));
                        })
                }
            }))
            .when(!disabled, |this| {
                this.when_some(self.on_click, |this, on_click| {
                    this.on_click(move |_, window, cx| {
                        if let Some(ix) = state.get() {
                            let mut checks = checks.clone();
                            checks[ix] = !checks[ix];
                            on_click(&checks, window, cx);
                        }
                    })
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IconName;

    #[gpui::test]
    fn test_toggle_builder(_cx: &mut gpui::TestAppContext) {
        let toggle = Toggle::new("complex-toggle")
            .label("Enable Feature")
            .icon(IconName::Check)
            .checked(true)
            .outline()
            .large()
            .disabled(false)
            .on_click(|_, _, _| {});

        assert_eq!(toggle.children.len(), 2); // label + icon
        assert!(toggle.checked);
        assert_eq!(toggle.variant, ToggleVariant::Outline);
        assert_eq!(toggle.size, Size::Large);
        assert!(!toggle.disabled);
        assert!(toggle.on_click.is_some());
    }

    #[gpui::test]
    fn test_toggle_group_builder(_cx: &mut gpui::TestAppContext) {
        let group = ToggleGroup::new("complex-group")
            .child(Toggle::new("toggle1").label("Option 1"))
            .child(Toggle::new("toggle2").label("Option 2").checked(true))
            .child(Toggle::new("toggle3").label("Option 3"))
            .outline()
            .large()
            .segmented()
            .disabled(false)
            .on_click(|_, _, _| {});

        assert_eq!(group.items.len(), 3);
        assert_eq!(group.variant, ToggleVariant::Outline);
        assert_eq!(group.size, Size::Large);
        assert!(group.segmented);
        assert!(!group.disabled);
        assert!(group.on_click.is_some());
    }
}
