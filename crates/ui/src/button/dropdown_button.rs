use gpui::{
    App, Context, Corner, Corners, Edges, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::{
    Disableable, IconName, Selectable, Sizable, Size, StyledExt as _,
    menu::{DropdownMenu, PopupMenu},
};

use super::{Button, ButtonRounded, ButtonVariant, ButtonVariants};

#[derive(IntoElement)]
pub struct DropdownButton {
    id: ElementId,
    style: StyleRefinement,
    button: Option<Button>,
    menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    selected: bool,
    disabled: bool,
    // The button props
    compact: bool,
    outline: bool,
    loading: bool,
    variant: ButtonVariant,
    size: Size,
    rounded: ButtonRounded,
    anchor: Corner,
}

impl DropdownButton {
    /// Create a new DropdownButton.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            button: None,
            menu: None,
            selected: false,
            disabled: false,
            compact: false,
            outline: false,
            loading: false,
            variant: ButtonVariant::default(),
            size: Size::default(),
            rounded: ButtonRounded::default(),
            anchor: Corner::TopRight,
        }
    }

    /// Set the left button of the dropdown button.
    pub fn button(mut self, button: Button) -> Self {
        self.button = Some(button);
        self
    }

    /// Set the dropdown menu of the button.
    pub fn dropdown_menu(
        mut self,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    /// Set the dropdown menu of the button with anchor corner.
    pub fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Corner>,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self.anchor = anchor.into();
        self
    }

    /// Set the rounded style of the button.
    pub fn rounded(mut self, rounded: impl Into<ButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    /// Set the button to compact style.
    ///
    /// See also: [`Button::compact`]
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Set the button to outline style.
    ///
    /// See also: [`Button::outline`]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Set the button to loading state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl Disableable for DropdownButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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
        self.size = size.into();
        self
    }
}

impl ButtonVariants for DropdownButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
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
        let rounded = self.variant.is_ghost() && !self.selected;

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
                        .loading(self.loading)
                        .selected(self.selected)
                        .disabled(self.disabled || self.loading)
                        .when(self.compact, |this| this.compact())
                        .when(self.outline, |this| this.outline())
                        .with_size(self.size)
                        .with_variant(self.variant),
                )
                .when_some(self.menu, |this, menu| {
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
                            .disabled(self.disabled || self.loading)
                            .when(self.compact, |this| this.compact())
                            .when(self.outline, |this| this.outline())
                            .with_size(self.size)
                            .with_variant(self.variant)
                            .dropdown_menu_with_anchor(self.anchor, menu),
                    )
                })
            })
    }
}
