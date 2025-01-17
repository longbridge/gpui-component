use gpui::{
    div, prelude::FluentBuilder as _, Corners, Div, Edges, ElementId, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement as _, Styled, WindowContext,
};
use std::{cell::Cell, rc::Rc};

use crate::{
    button::{Button, ButtonVariant, ButtonVariants},
    Disableable, Sizable, Size,
};

/// A ButtonGroup element, to wrap multiple buttons in a group.
#[derive(IntoElement)]
pub struct ButtonGroup {
    pub base: Div,
    id: ElementId,
    children: Vec<Button>,
    multiple: bool,
    disabled: bool,

    // The button props
    compact: Option<bool>,
    variant: Option<ButtonVariant>,
    size: Option<Size>,

    on_click: Option<Box<dyn Fn(&Vec<usize>, &mut WindowContext) + 'static>>,
}

impl Disableable for ButtonGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ButtonGroup {
    /// Creates a new ButtonGroup.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            children: Vec::new(),
            id: id.into(),
            variant: None,
            size: None,
            compact: None,
            multiple: false,
            disabled: false,
            on_click: None,
        }
    }

    /// Adds a button as a child to the ButtonGroup.
    pub fn child(mut self, child: Button) -> Self {
        self.children.push(child.disabled(self.disabled));
        self
    }

    /// With the multiple selection mode.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// With the compact mode for the ButtonGroup.
    pub fn compact(mut self) -> Self {
        self.compact = Some(true);
        self
    }

    /// Sets the on_click handler for the ButtonGroup.
    ///
    /// The handler first argument is a vector of the selected button indices.
    pub fn on_click(mut self, handler: impl Fn(&Vec<usize>, &mut WindowContext) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Sizable for ButtonGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl Styled for ButtonGroup {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl ButtonVariants for ButtonGroup {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        let children_len = self.children.len();
        let mut selected_ixs: Vec<usize> = Vec::new();
        let state = Rc::new(Cell::new(None));

        for (ix, child) in self.children.iter().enumerate() {
            if child.selected {
                selected_ixs.push(ix);
            }
        }

        self.base
            .id(self.id)
            .flex()
            .items_center()
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(child_index, child)| {
                        let state = Rc::clone(&state);
                        let child = if children_len == 1 {
                            child
                        } else if child_index == 0 {
                            // First
                            child
                                .border_corners(Corners {
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
                        } else if child_index == children_len - 1 {
                            // Last
                            child
                                .border_edges(Edges {
                                    left: false,
                                    top: true,
                                    right: true,
                                    bottom: true,
                                })
                                .border_corners(Corners {
                                    top_left: false,
                                    top_right: true,
                                    bottom_left: false,
                                    bottom_right: true,
                                })
                        } else {
                            // Middle
                            child
                                .border_corners(Corners::all(false))
                                .border_edges(Edges {
                                    left: false,
                                    top: true,
                                    right: true,
                                    bottom: true,
                                })
                        }
                        .stop_propagation(false)
                        .when_some(self.size, |this, size| this.with_size(size))
                        .when_some(self.variant, |this, variant| this.with_variant(variant))
                        .when_some(self.compact, |this, _| this.compact())
                        .on_click(move |_, _| {
                            state.set(Some(child_index));
                        });

                        child
                    }),
            )
            .when_some(
                self.on_click.filter(|_| !self.disabled),
                move |this, on_click| {
                    this.on_click(move |_, cx| {
                        let mut selected_ixs = selected_ixs.clone();
                        if let Some(ix) = state.get() {
                            if self.multiple {
                                if let Some(pos) = selected_ixs.iter().position(|&i| i == ix) {
                                    selected_ixs.remove(pos);
                                } else {
                                    selected_ixs.push(ix);
                                }
                            } else {
                                selected_ixs.clear();
                                selected_ixs.push(ix);
                            }
                        }

                        on_click(&selected_ixs, cx);
                    })
                },
            )
    }
}
