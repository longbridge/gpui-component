use gpui::{
    prelude::FluentBuilder, App, Context, Corner, Corners, Div, Edges, ElementId,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce, Styled, Window,
};

use crate::{
    h_flex,
    popup_menu::{PopupMenu, PopupMenuExt},
    IconName, Sizable, Size,
};

use super::{Button, ButtonRounded, ButtonVariant, ButtonVariants};

#[derive(IntoElement)]
pub struct DropdownButton {
    base: Div,
    id: ElementId,
    button: Option<Button>,
    popup_menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,

    // The button props
    compact: Option<bool>,
    outline: Option<bool>,
    variant: Option<ButtonVariant>,
    size: Option<Size>,
    rounded: ButtonRounded,
}

impl DropdownButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: h_flex(),
            id: id.into(),
            button: None,
            popup_menu: None,

            compact: None,
            outline: None,
            variant: None,
            size: None,
            rounded: ButtonRounded::Medium,
        }
    }

    pub fn button(mut self, button: Button) -> Self {
        self.button = Some(button);
        self
    }

    pub fn popup_menu(
        mut self,
        popup_menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.popup_menu = Some(Box::new(popup_menu));
        self
    }

    pub fn rounded(mut self, rounded: impl Into<ButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    pub fn compact(mut self) -> Self {
        self.compact = Some(true);
        self
    }

    pub fn outline(mut self) -> Self {
        self.outline = Some(true);
        self
    }
}

impl Styled for DropdownButton {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl Sizable for DropdownButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl ButtonVariants for DropdownButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl RenderOnce for DropdownButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base
            .id(self.id)
            .when_some(self.button, |this, button| {
                this.child(
                    button
                        .rounded(self.rounded)
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
                        .when_some(self.compact, |this, _| this.compact())
                        .when_some(self.outline, |this, _| this.outline())
                        .when_some(self.size, |this, size| this.with_size(size))
                        .when_some(self.variant, |this, variant| this.with_variant(variant)),
                )
                .when_some(self.popup_menu, |this, popup_menu| {
                    this.child(
                        Button::new("btn")
                            .icon(IconName::ChevronDown)
                            .rounded(self.rounded)
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
                            .when_some(self.compact, |this, _| this.compact())
                            .when_some(self.outline, |this, _| this.outline())
                            .when_some(self.size, |this, size| this.with_size(size))
                            .when_some(self.variant, |this, variant| this.with_variant(variant))
                            .popup_menu_with_anchor(Corner::TopRight, move |this, window, cx| {
                                popup_menu(this, window, cx)
                            }),
                    )
                })
            })
    }
}
