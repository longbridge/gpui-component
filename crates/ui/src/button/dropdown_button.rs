use gpui::Corners;
use gpui::{
    Anchor, App, Context, Edges, ElementId, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, StyleRefinement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::{
    Disableable, Selectable, Sizable, Size, StyledExt as _,
    menu::{DropdownMenu, PopupMenu},
};

use super::{Button, ButtonVariant, ButtonVariants};

/// A split button: an action button with an attached menu trigger.
///
/// The two halves stay visually joined, except for a `ghost` button that is not
/// selected — a toolbar reads better when an idle ghost pair looks like two
/// separate buttons.
///
#[derive(IntoElement)]
pub struct DropdownButton {
    id: ElementId,
    style: StyleRefinement,
    button: Option<Button>,
    menu:
        Option<Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static>>,
    selected: bool,
    disabled: bool,
    // The button props, applied to both halves. Unset means the inner
    // [`Button`] keeps whatever it was given.
    outline: bool,
    variant: Option<ButtonVariant>,
    size: Option<Size>,
    anchor: Anchor,
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
            outline: false,
            variant: None,
            size: None,
            anchor: Anchor::TopRight,
        }
    }

    fn effective_variant(&self) -> ButtonVariant {
        self.variant
            .or_else(|| self.button.as_ref().map(Button::variant))
            .unwrap_or_default()
    }

    fn effective_size(&self) -> Size {
        self.size
            .or_else(|| self.button.as_ref().map(Button::button_size))
            .unwrap_or_default()
    }

    /// Set the left button of the dropdown button.
    ///
    /// The button keeps its own label, icon, tooltip and click handler. A
    /// variant or size set on the [`DropdownButton`] applies to both halves and
    /// overrides the one set here. When either outer value is unset, this
    /// button's value becomes the shared value for both halves.
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
        anchor: impl Into<Anchor>,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self.anchor = anchor.into();
        self
    }

    /// Set the button to outline style.
    ///
    /// See also: [`Button::outline`]
    pub fn outline(mut self) -> Self {
        self.outline = true;
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
        debug_assert!(
            self.button.is_some() || self.menu.is_some(),
            "a DropdownButton needs a `button`, a `dropdown_menu`, or both"
        );

        let variant = self.effective_variant();
        let size = self.effective_size();
        let selected = self.selected || self.button.as_ref().is_some_and(Selectable::is_selected);
        let is_ghost = variant.is_ghost();
        let attached = !(is_ghost && !selected);

        div()
            .id(self.id)
            .h_flex()
            .refine_style(&self.style)
            .when_some(self.button, |this, button| {
                let disabled = self.disabled || button.is_disabled();
                this.child(
                    button
                        .border_corners(Corners {
                            top_left: true,
                            top_right: !attached,
                            bottom_left: true,
                            bottom_right: !attached,
                        })
                        .border_edges(Edges::all(true))
                        .selected(selected)
                        .disabled(disabled)
                        .when(self.outline, |this| this.outline())
                        .with_size(size)
                        .with_variant(variant),
                )
            })
            .when_some(self.menu, |this, menu| {
                this.child(
                    Button::new("popup")
                        .dropdown_caret(true)
                        .border_corners(Corners {
                            top_left: !attached,
                            top_right: true,
                            bottom_left: !attached,
                            bottom_right: true,
                        })
                        .border_edges(Edges {
                            left: !attached,
                            top: true,
                            right: true,
                            bottom: true,
                        })
                        .selected(selected)
                        .disabled(self.disabled)
                        .when(self.outline, |this| this.outline())
                        .with_size(size)
                        .with_variant(variant)
                        .dropdown_menu_with_anchor(self.anchor, menu),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn test_dropdown_button_builder(_cx: &mut gpui::TestAppContext) {
        let button = Button::new("inner").label("Action");
        let dropdown = DropdownButton::new("complex-dropdown")
            .button(button)
            .primary()
            .outline()
            .large()
            .disabled(false)
            .selected(false)
            .dropdown_menu_with_anchor(Anchor::BottomLeft, |menu, _, _| menu);

        assert!(dropdown.button.is_some());
        assert_eq!(dropdown.variant, Some(ButtonVariant::Primary));
        assert!(dropdown.outline);
        assert_eq!(dropdown.size, Some(Size::Large));
        assert!(!dropdown.disabled);
        assert!(!dropdown.selected);
        assert!(dropdown.menu.is_some());
        assert_eq!(dropdown.anchor, Anchor::BottomLeft);
    }

    /// An unset variant or size leaves the inner button's own to survive, so a
    /// caller can style the halves from either level.
    #[gpui::test]
    fn inner_button_keeps_its_own_variant_and_size(_cx: &mut gpui::TestAppContext) {
        let dropdown = DropdownButton::new("dropdown")
            .button(Button::new("inner").label("Action").danger().small())
            .dropdown_menu(|menu, _, _| menu);

        assert_eq!(dropdown.variant, None);
        assert_eq!(dropdown.size, None);
    }

    #[gpui::test]
    fn inner_ghost_becomes_the_split_variant(_cx: &mut gpui::TestAppContext) {
        let dropdown = DropdownButton::new("dropdown")
            .button(Button::new("inner").label("Action").ghost())
            .dropdown_menu(|menu, _, _| menu);

        assert_eq!(dropdown.effective_variant(), ButtonVariant::Ghost);
    }

    #[gpui::test]
    fn inner_size_becomes_the_split_size(_cx: &mut gpui::TestAppContext) {
        let dropdown = DropdownButton::new("dropdown")
            .button(Button::new("inner").label("Action").small())
            .dropdown_menu(|menu, _, _| menu);

        assert_eq!(dropdown.effective_size(), Size::Small);
    }
}
