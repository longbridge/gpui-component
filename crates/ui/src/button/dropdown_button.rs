use gpui::{
    Anchor, App, Context, ElementId, IntoElement, RenderOnce, StyleRefinement, Styled, Window,
    prelude::FluentBuilder,
};

use crate::{
    Disableable, Selectable, Sizable, Size, StyledExt as _,
    menu::{DropdownMenu, PopupMenu},
};

use super::{Button, ButtonGroup, ButtonVariant, ButtonVariants};

/// A split button: an action button with an attached menu trigger.
///
/// The two halves stay visually joined, except for a `ghost` button that is not
/// selected — a toolbar reads better when an idle ghost pair looks like two
/// separate buttons.
///
/// This is a thin wrapper over [`ButtonGroup`], which can hold a button that
/// opens a menu directly. Reach for the group when the split needs more than
/// two members, or when the halves need unrelated styling:
///
/// ```ignore
/// ButtonGroup::new("save")
///     .child(Button::new("save").label("Save"))
///     .child(
///         Button::new("save-options")
///             .dropdown_caret(true)
///             .dropdown_menu(|menu, _, _| menu.menu("Save as…", Box::new(SaveAs))),
///     )
/// ```
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

    /// Set the left button of the dropdown button.
    ///
    /// The button keeps its own label, icon, tooltip and click handler. A
    /// variant or size set on the [`DropdownButton`] applies to both halves and
    /// overrides the one set here; leave it unset to keep this button's own.
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

        let is_ghost = self
            .variant
            .as_ref()
            .is_some_and(|variant| variant.is_ghost());

        ButtonGroup::new(self.id)
            // The halves run an action and open a menu; neither is a toggle,
            // and `selected` only styles them.
            .no_toggle()
            .attached(!(is_ghost && !self.selected))
            .disabled(self.disabled)
            .when(self.outline, |this| this.outline())
            .when_some(self.variant, |this, variant| this.with_variant(variant))
            .when_some(self.size, |this, size| this.with_size(size))
            .refine_style(&self.style)
            .when_some(self.button, |this, button| {
                this.child(button.selected(self.selected))
            })
            .when_some(self.menu, |this, menu| {
                this.child(
                    Button::new("popup")
                        .dropdown_caret(true)
                        .selected(self.selected)
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
}
