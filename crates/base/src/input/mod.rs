use crate::{StyledExt as _, theme::ActiveTheme as _};
use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
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
pub use native::{NativeMenu, NativeMenuItem};
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use search::{SearchMatcher, SearchSession};
pub use state::*;

/// The foundational input frame.
///
/// It intentionally owns only input semantics, interaction forwarding, normal
/// children, and the minimal semantic border/radius requested of Base inputs.
/// Applications remain responsible for layout, padding, typography, background,
/// adornments, editor rendering, and richer focus presentation.
#[derive(IntoElement)]
pub struct Input {
    base: gpui::Stateful<Div>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    appearance: bool,
    bordered: bool,
    focus_bordered: bool,
    focused: bool,
    role: crate::RoleOverride,
}

impl Input {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            style: StyleRefinement::default(),
            children: Vec::new(),
            appearance: true,
            bordered: true,
            focus_bordered: true,
            focused: false,
            role: crate::RoleOverride::Implicit,
        }
    }
    pub fn role(mut self, role: impl Into<crate::RoleOverride>) -> Self {
        self.role = role.into();
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn focus_bordered(mut self, focus_bordered: bool) -> Self {
        self.focus_bordered = focus_bordered;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.base = self.base.aria_label(label.into());
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().tokens;

        self.base
            .when_some(self.role.resolve(|| Role::TextInput), |this, role| {
                this.role(role)
            })
            .when(self.appearance, |this| {
                this.rounded(tokens.radius.md).when(self.bordered, |this| {
                    this.border_1()
                        .border_color(tokens.colors.input)
                        .when(self.focused && self.focus_bordered, |this| {
                            this.border_color(tokens.colors.ring)
                        })
                })
            })
            .children(self.children)
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_accepts_application_owned_content_and_style() {
        let _ = Input::new("input")
            .appearance(true)
            .bordered(true)
            .focus_bordered(true)
            .focused(false)
            .child("value")
            .opacity(0.8);
    }
}
