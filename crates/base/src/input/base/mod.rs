use crate::{StateStyle, StyledExt as _};
use gpui::{
    AbsoluteLength, AnyElement, App, DefiniteLength, Div, ElementId, FontWeight,
    InteractiveElement, Interactivity, IntoElement, ParentElement, Refineable as _, RenderOnce,
    Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, TextStyle, Window,
    div, prelude::FluentBuilder as _,
};

/// The font an input paints its text with.
///
/// The four settings mirror the font group of a code editor's options, as
/// Monaco spells it: `fontFamily`, `fontSize`, `fontWeight`, and `lineHeight`.
/// Anything left unset falls through to the ambient text style, so an input
/// keeps inheriting its surroundings until something is pinned here.
///
/// ```
/// use gpui::{px, relative};
/// use gpui_base::input::InputFont;
///
/// let font = InputFont::new()
///     .with_family("JetBrains Mono")
///     .with_size(px(13.))
///     .with_line_height(relative(1.5));
///
/// assert_eq!(font.family(), Some("JetBrains Mono"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputFont {
    family: Option<SharedString>,
    size: Option<AbsoluteLength>,
    weight: Option<FontWeight>,
    line_height: Option<DefiniteLength>,
}

impl InputFont {
    pub fn new() -> Self {
        Self::default()
    }

    /// The family to shape the text with, a monospace one for source code.
    pub fn with_family(mut self, family: impl Into<SharedString>) -> Self {
        self.family = Some(family.into());
        self
    }

    /// The size to paint the text at.
    ///
    /// A relative line height follows it, so the rows stay in proportion.
    pub fn with_size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// The height of one row: a fraction of the font size, or an absolute length.
    pub fn with_line_height(mut self, line_height: impl Into<DefiniteLength>) -> Self {
        self.line_height = Some(line_height.into());
        self
    }

    pub fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    pub fn size(&self) -> Option<AbsoluteLength> {
        self.size
    }

    pub fn weight(&self) -> Option<FontWeight> {
        self.weight
    }

    pub fn line_height(&self) -> Option<DefiniteLength> {
        self.line_height
    }

    /// Lay this font over an ambient text style.
    pub fn resolve(&self, mut style: TextStyle) -> TextStyle {
        if let Some(family) = self.family.clone() {
            style.font_family = family;
        }
        if let Some(size) = self.size {
            style.font_size = size;
        }
        if let Some(weight) = self.weight {
            style.font_weight = weight;
        }
        if let Some(line_height) = self.line_height {
            style.line_height = line_height;
        }

        style
    }
}

/// So that a font can be built with `when` and `when_some`, the way an element is.
impl gpui::prelude::FluentBuilder for InputFont {}

/// What the input can offer to its context menu, at the moment it is opened.
///
/// Built by the input and read by the menu, the fields are private and reached
/// through the methods below, so that a new capability can be added without
/// breaking the menu builders.
///
/// ```
/// use gpui_base::input::InputContextMenuCapabilities;
///
/// let capabilities = InputContextMenuCapabilities::new()
///     .code_editor(true)
///     .selection(true);
///
/// assert!(capabilities.is_editable());
/// assert!(capabilities.has_selection());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputContextMenuCapabilities {
    disabled: bool,
    readonly: bool,
    code_editor: bool,
    selection: bool,
    go_to_definition: bool,
    code_actions: bool,
}

impl InputContextMenuCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn code_editor(mut self, code_editor: bool) -> Self {
        self.code_editor = code_editor;
        self
    }

    /// Set whether the input has a non-empty selection.
    pub fn selection(mut self, selection: bool) -> Self {
        self.selection = selection;
        self
    }

    pub fn go_to_definition(mut self, go_to_definition: bool) -> Self {
        self.go_to_definition = go_to_definition;
        self
    }

    pub fn code_actions(mut self, code_actions: bool) -> Self {
        self.code_actions = code_actions;
        self
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    /// Returns true if the user is allowed to change the text.
    ///
    /// The items that write to the input (Cut, Paste, Code Actions) belong to
    /// this, the reading ones (Copy, Go to Definition) do not.
    pub fn is_editable(&self) -> bool {
        !self.disabled && !self.readonly
    }

    pub fn is_code_editor(&self) -> bool {
        self.code_editor
    }

    pub fn has_selection(&self) -> bool {
        self.selection
    }

    pub fn can_go_to_definition(&self) -> bool {
        self.go_to_definition
    }

    pub fn has_code_actions(&self) -> bool {
        self.code_actions
    }
}

/// The foundational input frame.
///
/// It intentionally owns only input semantics, interaction forwarding, and
/// normal children. Applications remain responsible for all presentation.
#[derive(IntoElement)]
pub struct InputBase {
    base: gpui::Stateful<Div>,
    style: StyleRefinement,
    semantic_styles: InputStyles,
    children: Vec<AnyElement>,
    focused: bool,
    disabled: bool,
    role: crate::RoleOverride,
}

impl InputBase {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            semantic_styles: InputStyles::default(),
            children: Vec::new(),
            focused: false,
            disabled: false,
            role: crate::RoleOverride::Implicit,
        }
    }
    pub fn role(mut self, role: impl Into<crate::RoleOverride>) -> Self {
        self.role = role.into();
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn styles(mut self, build: impl FnOnce(InputStyles) -> InputStyles) -> Self {
        self.semantic_styles = build(self.semantic_styles);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.base = self.base.aria_label(label.into());
        self
    }

    fn resolved_style(&self) -> StyleRefinement {
        crate::state_style::resolve_style(
            &self.style,
            [
                self.focused.then_some(&self.semantic_styles.focused),
                self.disabled.then_some(&self.semantic_styles.disabled),
            ]
            .into_iter()
            .flatten(),
        )
    }
}

#[derive(Default)]
pub struct InputStyles {
    focused: StyleRefinement,
    disabled: StyleRefinement,
}

impl InputStyles {
    pub fn focused(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.focused
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }

    pub fn disabled(mut self, build: impl FnOnce(StateStyle) -> StateStyle) -> Self {
        self.disabled
            .refine(&build(StateStyle::default()).into_refinement());
        self
    }
}

impl Styled for InputBase {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for InputBase {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for InputBase {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for InputBase {}

impl RenderOnce for InputBase {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let style = self.resolved_style();
        self.base
            .when_some(self.role.resolve(|| Role::TextInput), |this, role| {
                this.role(role)
            })
            .children(self.children)
            .refine_style(&style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_accepts_application_owned_content_and_style() {
        let _ = InputBase::new("input")
            .focused(true)
            .disabled(false)
            .styles(|styles| {
                styles
                    .focused(|style| style.border_1())
                    .disabled(|style| style.opacity(0.5))
            })
            .child("value")
            .opacity(0.8);
    }

    #[test]
    fn semantic_state_styles_override_the_normal_style() {
        let focused = InputBase::new("focused")
            .focused(true)
            .border_color(gpui::red())
            .styles(|styles| styles.focused(|style| style.border_color(gpui::blue())));
        assert_eq!(focused.resolved_style().border_color, Some(gpui::blue()));

        let disabled = InputBase::new("disabled")
            .focused(true)
            .disabled(true)
            .opacity(1.)
            .styles(|styles| {
                styles
                    .focused(|style| style.opacity(0.8))
                    .disabled(|style| style.opacity(0.5))
            });
        assert_eq!(disabled.resolved_style().opacity, Some(0.5));
    }
}
