use crate::{StateStyle, StyledExt as _};
use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    Refineable as _, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};

/// Character used by masked editor modes.
pub(crate) const MASK_CHAR: char = '•';

pub(crate) mod blink_cursor;
mod change;
mod cursor;
mod decorations;
mod diagnostics;
mod display_map;
mod element;
mod highlighting;
mod indent;
mod layout;
mod lsp;
mod mask_pattern;
mod mode;
mod movement;
mod native;
mod rope_ext;
mod search;
mod selection;
mod state;

pub(crate) fn init(cx: &mut App) {
    state::init(cx);
}

pub use crate::number_input::{NumberInputEvent, NumberStep};
pub use cursor::Selection;
pub use decorations::{TextDecoration, TextDecorationCollection};
pub use diagnostics::{
    Diagnostic, DiagnosticEntry, DiagnosticRelatedInformation, DiagnosticSet, DiagnosticSeverity,
    DiagnosticSummary, DiagnosticTag, RelatedInformation,
};
pub use display_map::{BufferPoint, DisplayMap, DisplayPoint, FoldRange, WrappingIndent};
pub use highlighting::{
    DiagnosticColors, FoldIconRenderer, HighlightStyleResolver, InputEditorStyle, InputHighlighter,
    InputHighlighterFactory, SharedHighlightStyleResolver,
};
pub use indent::TabSize;
pub use lsp::{
    CodeActionItem, CodeActionMenuState, CodeActionProvider, CompletionMenuOptions,
    CompletionMenuState, CompletionProvider, DefinitionProvider, DocumentColorProvider,
    DocumentRangeSemanticTokensProvider, HoverPopoverState, HoverProvider, InputOverlayKind, Lsp,
    ShowDocumentHandler,
};
pub(super) use lsp::{HoverDefinition, InlineCompletion};
pub use lsp_types::Position;
pub use mask_pattern::MaskPattern;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub use native::set_text_content_type;
pub use native::{NativeMenu, NativeMenuItem};
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use search::{SearchMatcher, SearchSession};
pub use state::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputContextMenuCapabilities {
    pub disabled: bool,
    pub code_editor: bool,
    pub selection: bool,
    pub go_to_definition: bool,
    pub code_actions: bool,
}

/// The foundational input frame.
///
/// It intentionally owns only input semantics, interaction forwarding, and
/// normal children. Applications remain responsible for all presentation.
#[derive(IntoElement)]
pub struct Input {
    base: gpui::Stateful<Div>,
    style: StyleRefinement,
    semantic_styles: InputStyles,
    children: Vec<AnyElement>,
    focused: bool,
    disabled: bool,
    role: crate::RoleOverride,
}

impl Input {
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

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Input {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Input {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Input {}

impl RenderOnce for Input {
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
        let _ = Input::new("input")
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
        let focused = Input::new("focused")
            .focused(true)
            .border_color(gpui::red())
            .styles(|styles| styles.focused(|style| style.border_color(gpui::blue())));
        assert_eq!(focused.resolved_style().border_color, Some(gpui::blue()));

        let disabled = Input::new("disabled")
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
