use gpui::{
    div, prelude::FluentBuilder, App, Context, Corner, Corners, Edges, ElementId,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window,
};

use crate::{
    popup_menu::{PopupMenu, PopupMenuExt},
    IconName, Selectable, Sizable, Size, StyledExt as _,
};

use super::{Button, ButtonRounded, ButtonVariant, ButtonVariants};

#[derive(IntoElement)]
pub struct DropdownButton {
    id: ElementId,
    style: StyleRefinement,
    button: Option<Button>,
    popup_menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    selected: bool,
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
            id: id.into(),
            style: StyleRefinement::default(),
            button: None,
            popup_menu: None,
            selected: false,
            compact: None,
            outline: None,
            variant: None,
            size: None,
            rounded: ButtonRounded::default(),
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
        &mut self.style
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

impl Selectable for DropdownButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for DropdownButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let rounded = self
            .variant
            .map(|variant| variant.is_ghost() && !self.selected)
            .unwrap_or(false);

        div()
            .id(self.id)
            .h_flex()
            .refine_style(&self.style)
            .when_some(self.button, |this, button| {
                this.child(
                    button
                        .rounded(self.rounded)
                        .border_corners(Corners {
                            top_left: true,
                            top_right: rounded,
                            bottom_left: true,
                            bottom_right: rounded,
                        })
                        .border_edges(Edges {
                            left: true,
                            top: true,
                            right: true,
                            bottom: true,
                        })
                        .selected(self.selected)
                        .when_some(self.compact, |this, _| this.compact())
                        .when_some(self.outline, |this, _| this.outline())
                        .when_some(self.size, |this, size| this.with_size(size))
                        .when_some(self.variant, |this, variant| this.with_variant(variant)),
                )
                .when_some(self.popup_menu, |this, popup_menu| {
                    this.child(
                        Button::new("popup")
                            .icon(IconName::ChevronDown)
                            .rounded(self.rounded)
                            .border_edges(Edges {
                                left: rounded,
                                top: true,
                                right: true,
                                bottom: true,
                            })
                            .border_corners(Corners {
                                top_left: rounded,
                                top_right: true,
                                bottom_left: rounded,
                                bottom_right: true,
                            })
                            .selected(self.selected)
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
