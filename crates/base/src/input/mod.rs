use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, Interactivity, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
};
use std::rc::Rc;

use crate::{StyledExt as _, theme::ActiveTheme as _};

/// Character used by masked editor modes.
pub(crate) const MASK_CHAR: char = '•';

pub(crate) mod blink_cursor;
mod change;
mod cursor;
mod decorations;
mod diagnostics;
mod display_map;
mod editing_indent;
mod element;
mod highlighting;
mod indent;
mod layout;
mod lsp;
mod mask_pattern;
mod mode;
mod movement;
#[cfg(target_os = "macos")]
mod native;
mod rope_ext;
mod search;
mod selection;
mod state;

pub(crate) fn init(cx: &mut App) {
    state::init(cx);
}

pub use crate::number_input::NumberInputEvent;
pub use cursor::Selection;
pub use decorations::{TextDecoration, TextDecorationCollection};
pub use diagnostics::{
    Diagnostic, DiagnosticEntry, DiagnosticRelatedInformation, DiagnosticSet, DiagnosticSeverity,
    DiagnosticSummary, DiagnosticTag, RelatedInformation,
};
pub use display_map::{BufferPoint, DisplayMap, DisplayPoint, FoldRange, WrappingIndent};
pub use highlighting::{
    FoldIconRenderer, HighlightStyleResolver, InputEditorStyle, InputHighlighter,
    InputHighlighterFactory, SharedHighlightStyleResolver,
};
pub use indent::TabSize;
pub use lsp::{
    CodeActionItem, CodeActionProvider, CodeActionSession, CompletionMenuOptions,
    CompletionProvider, CompletionSession, DefinitionProvider, DocumentColorProvider,
    DocumentRangeSemanticTokensProvider, HoverProvider, HoverSession, InputOverlayKind, Lsp,
    ShowDocumentHandler,
};
pub(super) use lsp::{HoverDefinition, InlineCompletion};
pub use lsp_types::Position;
pub use mask_pattern::MaskPattern;
#[cfg(target_os = "macos")]
#[doc(hidden)]
pub use native::set_text_content_type;
pub use rope_ext::{InputEdit, Point, RopeExt, RopeLines};
pub use ropey::Rope;
pub use search::{SearchMatcher, SearchSession};
pub use state::*;

/// Presentation scale used by the editor's geometry calculations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Strategy used by numeric editors when stepping their value.
#[derive(Clone)]
pub enum NumberStep {
    Fixed(f64),
    ByValue(Rc<dyn Fn(f64, crate::StepAction, &mut gpui::Context<InputState>) -> f64>),
}

impl NumberStep {
    pub fn by_value(
        f: impl Fn(f64, crate::StepAction, &mut gpui::Context<InputState>) -> f64 + 'static,
    ) -> Self {
        Self::ByValue(Rc::new(f))
    }

    pub(crate) fn value(
        &self,
        current: f64,
        action: crate::StepAction,
        cx: &mut gpui::Context<InputState>,
    ) -> f64 {
        match self {
            Self::Fixed(step) => *step,
            Self::ByValue(f) => f(current, action, cx),
        }
    }
}

impl From<f64> for NumberStep {
    fn from(step: f64) -> Self {
        Self::Fixed(step)
    }
}

/// Presentation-independent context-menu model produced by the editor.
#[derive(Default)]
pub struct NativeMenu {
    pub items: Vec<NativeMenuItem>,
}

pub enum NativeMenuItem {
    Separator,
    Action {
        label: SharedString,
        disabled: bool,
        action: Box<dyn gpui::Action>,
    },
}

impl NativeMenu {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn menu(self, label: impl Into<SharedString>, action: Box<dyn gpui::Action>) -> Self {
        self.menu_with_disabled(label, false, action)
    }
    pub fn menu_with_disabled(
        mut self,
        label: impl Into<SharedString>,
        disabled: bool,
        action: Box<dyn gpui::Action>,
    ) -> Self {
        self.items.push(NativeMenuItem::Action {
            label: label.into(),
            disabled,
            action,
        });
        self
    }
    pub fn separator(mut self) -> Self {
        self.items.push(NativeMenuItem::Separator);
        self
    }
}

/// Semantic content type used by text inputs, password managers, and autofill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContentType {
    Name,
    NamePrefix,
    GivenName,
    MiddleName,
    FamilyName,
    NameSuffix,
    Nickname,
    JobTitle,
    OrganizationName,
    Location,
    FullStreetAddress,
    StreetAddressLine1,
    StreetAddressLine2,
    AddressCity,
    AddressState,
    AddressCityAndState,
    Sublocality,
    CountryName,
    PostalCode,
    TelephoneNumber,
    EmailAddress,
    Url,
    CreditCardNumber,
    CreditCardName,
    CreditCardGivenName,
    CreditCardMiddleName,
    CreditCardFamilyName,
    CreditCardSecurityCode,
    CreditCardExpiration,
    CreditCardExpirationMonth,
    CreditCardExpirationYear,
    CreditCardType,
    Username,
    Password,
    NewPassword,
    OneTimeCode,
    ShipmentTrackingNumber,
    FlightNumber,
    DateTime,
    Birthdate,
    BirthdateDay,
    BirthdateMonth,
    BirthdateYear,
    CellularEid,
    CellularImei,
}

impl InputContentType {
    pub fn accessibility_role(self) -> Role {
        match self {
            Self::TelephoneNumber => Role::PhoneNumberInput,
            Self::EmailAddress => Role::EmailInput,
            Self::Url => Role::UrlInput,
            Self::Password | Self::NewPassword => Role::PasswordInput,
            Self::DateTime => Role::DateTimeInput,
            Self::Birthdate => Role::DateInput,
            _ => Role::TextInput,
        }
    }

    pub fn exposes_accessibility_value(self) -> bool {
        !matches!(self, Self::Password | Self::NewPassword)
    }

    #[cfg(target_os = "macos")]
    #[doc(hidden)]
    pub const fn ns_text_content_type(self) -> Option<&'static str> {
        match self {
            Self::Name => Some("name"),
            Self::NamePrefix => Some("honorific-prefix"),
            Self::GivenName => Some("given-name"),
            Self::MiddleName => Some("additional-name"),
            Self::FamilyName => Some("family-name"),
            Self::NameSuffix => Some("honorific-suffix"),
            Self::Nickname => Some("nickname"),
            Self::JobTitle => Some("organization-title"),
            Self::OrganizationName => Some("organization"),
            Self::Location => Some("location"),
            Self::FullStreetAddress => Some("street-address"),
            Self::StreetAddressLine1 => Some("address-line1"),
            Self::StreetAddressLine2 => Some("address-line2"),
            Self::AddressCity => Some("address-level2"),
            Self::AddressState => Some("address-level1"),
            Self::AddressCityAndState => Some("address-level1+2"),
            Self::Sublocality => Some("address-level3"),
            Self::CountryName => Some("country-name"),
            Self::PostalCode => Some("postal-code"),
            Self::TelephoneNumber => Some("tel"),
            Self::EmailAddress => Some("email"),
            Self::Url => Some("url"),
            Self::CreditCardNumber => Some("cc-number"),
            Self::CreditCardName => Some("cc-name"),
            Self::CreditCardGivenName => Some("cc-given-name"),
            Self::CreditCardMiddleName => Some("cc-additional-name"),
            Self::CreditCardFamilyName => Some("cc-family-name"),
            Self::CreditCardSecurityCode => Some("cc-csc"),
            Self::CreditCardExpiration => Some("cc-exp"),
            Self::CreditCardExpirationMonth => Some("cc-exp-month"),
            Self::CreditCardExpirationYear => Some("cc-exp-year"),
            Self::CreditCardType => Some("cc-type"),
            Self::Username => Some("username"),
            Self::Password => Some("password"),
            Self::NewPassword => Some("new-password"),
            Self::OneTimeCode => Some("one-time-code"),
            Self::ShipmentTrackingNumber => Some("shipment-tracking-number"),
            Self::FlightNumber => Some("flight-number"),
            Self::DateTime => Some("date-time"),
            Self::Birthdate => Some("bday"),
            Self::BirthdateDay => Some("bday-day"),
            Self::BirthdateMonth => Some("bday-month"),
            Self::BirthdateYear => Some("bday-year"),
            Self::CellularEid | Self::CellularImei => None,
        }
    }
}

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
    fn content_types_project_accessibility_without_exposing_password_values() {
        assert_eq!(
            InputContentType::EmailAddress.accessibility_role(),
            Role::EmailInput
        );
        assert!(!InputContentType::Password.exposes_accessibility_value());
        assert!(InputContentType::Username.exposes_accessibility_value());
    }

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
