use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, DefiniteLength, Edges, EdgesRefinement, Entity, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Rems, RenderOnce, StyleRefinement, Styled,
    TextAlign, Window, div, px, relative,
};

use crate::button::{Button, ButtonVariants as _};
use crate::input::clear_button;
use crate::native_menu::NativeMenu;
use crate::spinner::Spinner;
use crate::{ActiveTheme, Colorize, v_flex};
use crate::{IconName, Size};
use crate::{Selectable, StyledExt, h_flex};
use crate::{Sizable, StyleSized};

use super::{InputState, element::EditorScrollbar};

/// Returns `(background, foreground)` colors for input-like components.
pub(crate) fn input_style(disabled: bool, cx: &App) -> (Hsla, Hsla) {
    if disabled {
        (
            cx.theme().input.mix_oklab(cx.theme().transparent, 0.8),
            cx.theme().muted_foreground,
        )
    } else {
        (cx.theme().input_background(), cx.theme().foreground)
    }
}

/// Semantic content type for an [`Input`].
///
/// These variants mirror Swift's text content types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputContentType {
    /// A person's full name.
    Name,
    /// A name prefix, such as Mr. or Dr.
    NamePrefix,
    /// A person's given name.
    GivenName,
    /// A person's middle name.
    MiddleName,
    /// A person's family name.
    FamilyName,
    /// A name suffix, such as Jr. or PhD.
    NameSuffix,
    /// A nickname.
    Nickname,
    /// A job title.
    JobTitle,
    /// An organization or company name.
    OrganizationName,
    /// A location name.
    Location,
    /// A full street address.
    FullStreetAddress,
    /// The first line of a street address.
    StreetAddressLine1,
    /// The second line of a street address.
    StreetAddressLine2,
    /// A city or locality.
    AddressCity,
    /// A state, province, or region.
    AddressState,
    /// A combined city and state.
    AddressCityAndState,
    /// A sublocality, district, or neighborhood.
    Sublocality,
    /// A country name.
    CountryName,
    /// A postal or ZIP code.
    PostalCode,
    /// A telephone number.
    TelephoneNumber,
    /// An email address.
    EmailAddress,
    /// A URL.
    Url,
    /// A credit card number.
    CreditCardNumber,
    /// The full name on a credit card.
    CreditCardName,
    /// The given name on a credit card.
    CreditCardGivenName,
    /// The middle name on a credit card.
    CreditCardMiddleName,
    /// The family name on a credit card.
    CreditCardFamilyName,
    /// The security code on a credit card.
    CreditCardSecurityCode,
    /// A credit card expiration date.
    CreditCardExpiration,
    /// A credit card expiration month.
    CreditCardExpirationMonth,
    /// A credit card expiration year.
    CreditCardExpirationYear,
    /// A credit card type.
    CreditCardType,
    /// A username or account identifier.
    Username,
    /// The password for the account identified by the username field.
    Password,
    /// A new password, such as during sign up or password reset.
    NewPassword,
    /// A one-time verification code.
    OneTimeCode,
    /// A parcel shipment tracking number.
    ShipmentTrackingNumber,
    /// An airline flight number.
    FlightNumber,
    /// A date, time, or duration.
    DateTime,
    /// A birthdate.
    Birthdate,
    /// A birthdate day.
    BirthdateDay,
    /// A birthdate month.
    BirthdateMonth,
    /// A birthdate year.
    BirthdateYear,
    /// An eSIM EID.
    CellularEid,
    /// A cellular IMEI.
    CellularImei,
}

impl InputContentType {
    #[cfg(target_os = "macos")]
    pub(crate) const fn ns_text_content_type(self) -> Option<&'static str> {
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

/// A text input element bind to an [`InputState`].
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    style: StyleRefinement,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    height: Option<DefiniteLength>,
    appearance: bool,
    cleanable: bool,
    mask_toggle: bool,
    disabled: bool,
    bordered: bool,
    focus_bordered: bool,
    tab_index: isize,
    selected: bool,
    content_type: Option<InputContentType>,

    /// An optional context menu builder to allow a custom context menu on the input.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Sizable for Input {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Selectable for Input {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Input {
    /// Create a new [`Input`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
            prefix: None,
            suffix: None,
            height: None,
            appearance: true,
            cleanable: false,
            mask_toggle: false,
            disabled: false,
            bordered: true,
            focus_bordered: true,
            tab_index: 0,
            selected: false,
            content_type: None,
            context_menu_builder: None,
        }
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set full height of the input (Multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self
    }

    /// Set height of the input (Multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set the appearance of the input field, if false the input field will no border, background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set the bordered for the input, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set focus border for the input, default is true.
    pub fn focus_bordered(mut self, bordered: bool) -> Self {
        self.focus_bordered = bordered;
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set to enable toggle button for password mask state.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Set the semantic content type for password managers and autofill.
    ///
    /// This is a component-level semantic hint. It does not change the text
    /// value or masked rendering state.
    pub fn content_type(mut self, content_type: InputContentType) -> Self {
        self.content_type = Some(content_type);
        self
    }

    /// Set to disable the input field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the tab index for the input, default is 0.
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    /// Sets a custom context menu builder for the input, shown as a native OS menu.
    ///
    /// If set, this overrides the built-in right-click context menu.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }

    fn render_toggle_mask_button(state: &Entity<InputState>, cx: &App) -> impl IntoElement {
        let masked = state.read(cx).masked;
        Button::new("toggle-mask")
            .icon(if masked {
                IconName::Eye
            } else {
                IconName::EyeOff
            })
            .xsmall()
            .ghost()
            .tab_stop(false)
            .on_click({
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_masked(!state.masked, window, cx);
                    })
                }
            })
    }

    /// This method must after the refine_style.
    fn render_editor(
        paddings: EdgesRefinement<DefiniteLength>,
        input_state: &Entity<InputState>,
        state: &InputState,
        window: &Window,
    ) -> impl IntoElement {
        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();

        let paddings = Edges {
            left: paddings
                .left
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            right: paddings
                .right
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            top: paddings
                .top
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            bottom: paddings
                .bottom
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
        };

        state.editor_scrollbar_paddings.set(paddings);
        state.editor_scrollbar_snapshot.set(None);

        v_flex()
            .size_full()
            .children(state.search_panel.clone())
            .child(
                div()
                    .relative()
                    .flex_1()
                    .child(input_state.clone())
                    .child(EditorScrollbar::new(input_state.clone())),
            )
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        const LINE_HEIGHT: Rems = Rems(1.25);
        let text_align = self.style.text.text_align.unwrap_or(TextAlign::Left);

        self.state.update(cx, |state, _| {
            state.context_menu_builder = self.context_menu_builder.clone();
            state.disabled = self.disabled;
            state.size = self.size;

            // Only for single line mode
            if state.mode.is_single_line() {
                state.text_align = text_align;
            }
        });

        let state = self.state.read(cx);
        let content_type = self.content_type;
        let disabled = self.disabled;
        let focused = state.focus_handle.is_focused(window) && !state.disabled;
        #[cfg(target_os = "macos")]
        if focused {
            super::native::set_text_content_type(window, content_type);
        }

        let gap_x = match self.size {
            Size::Small => px(4.),
            Size::Large => px(8.),
            _ => px(6.),
        };

        let (bg, _) = input_style(state.disabled, cx);
        let bg = if state.mode.is_code_editor() {
            cx.theme().editor_background()
        } else {
            bg
        };
        let bg = if state.disabled { bg.opacity(0.5) } else { bg };
        let border_color = if state.disabled {
            cx.theme().input.opacity(0.5)
        } else {
            cx.theme().input
        };

        let prefix = self.prefix;
        let suffix = self.suffix;
        let show_clear_button = self.cleanable
            && !state.disabled
            && !state.loading
            && state.text.len() > 0
            && state.mode.is_single_line();
        let has_suffix = suffix.is_some() || state.loading || self.mask_toggle || show_clear_button;

        div()
            .id(("input", self.state.entity_id()))
            .flex()
            .key_context(crate::input::CONTEXT)
            .track_focus(&state.focus_handle.clone())
            .tab_index(self.tab_index)
            .when(!state.disabled, |this| {
                this.on_action(window.listener_for(&self.state, InputState::backspace))
                    .on_action(window.listener_for(&self.state, InputState::delete))
                    .on_action(
                        window.listener_for(&self.state, InputState::delete_to_beginning_of_line),
                    )
                    .on_action(window.listener_for(&self.state, InputState::delete_to_end_of_line))
                    .on_action(window.listener_for(&self.state, InputState::delete_previous_word))
                    .on_action(window.listener_for(&self.state, InputState::delete_next_word))
                    .on_action(window.listener_for(&self.state, InputState::enter))
                    .on_action(window.listener_for(&self.state, InputState::escape))
                    .on_action(window.listener_for(&self.state, InputState::paste))
                    .on_action(window.listener_for(&self.state, InputState::cut))
                    .on_action(window.listener_for(&self.state, InputState::undo))
                    .on_action(window.listener_for(&self.state, InputState::redo))
                    .when(state.mode.is_multi_line(), |this| {
                        this.on_action(window.listener_for(&self.state, InputState::indent_inline))
                            .on_action(window.listener_for(&self.state, InputState::outdent_inline))
                            .on_action(window.listener_for(&self.state, InputState::indent_block))
                            .on_action(window.listener_for(&self.state, InputState::outdent_block))
                    })
                    .on_action(
                        window.listener_for(&self.state, InputState::on_action_toggle_code_actions),
                    )
            })
            .on_action(window.listener_for(&self.state, InputState::left))
            .on_action(window.listener_for(&self.state, InputState::right))
            .on_action(window.listener_for(&self.state, InputState::select_left))
            .on_action(window.listener_for(&self.state, InputState::select_right))
            .when(state.mode.is_multi_line(), |this| {
                let result = this
                    .on_action(window.listener_for(&self.state, InputState::up))
                    .on_action(window.listener_for(&self.state, InputState::down))
                    .on_action(window.listener_for(&self.state, InputState::select_up))
                    .on_action(window.listener_for(&self.state, InputState::select_down))
                    .on_action(window.listener_for(&self.state, InputState::page_up))
                    .on_action(window.listener_for(&self.state, InputState::page_down));

                let result = result.on_action(
                    window.listener_for(&self.state, InputState::on_action_go_to_definition),
                );

                result
            })
            .on_action(window.listener_for(&self.state, InputState::select_all))
            .on_action(window.listener_for(&self.state, InputState::select_to_start_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_end_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::home))
            .on_action(window.listener_for(&self.state, InputState::end))
            .on_action(window.listener_for(&self.state, InputState::move_to_start))
            .on_action(window.listener_for(&self.state, InputState::move_to_end))
            .on_action(window.listener_for(&self.state, InputState::move_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::move_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_start))
            .on_action(window.listener_for(&self.state, InputState::select_to_end))
            .on_action(window.listener_for(&self.state, InputState::show_character_palette))
            .on_action(window.listener_for(&self.state, InputState::copy))
            .on_action(window.listener_for(&self.state, InputState::on_action_search))
            .on_key_down(window.listener_for(&self.state, InputState::on_key_down))
            .on_mouse_down(MouseButton::Left, {
                let state = self.state.clone();
                move |event, window, cx| {
                    #[cfg(target_os = "macos")]
                    if !disabled {
                        super::native::set_text_content_type(window, content_type);
                    }

                    state.update(cx, |state, cx| state.on_mouse_down(event, window, cx));
                }
            })
            .on_mouse_down(MouseButton::Right, {
                let state = self.state.clone();
                move |event, window, cx| {
                    #[cfg(target_os = "macos")]
                    if !disabled {
                        super::native::set_text_content_type(window, content_type);
                    }

                    state.update(cx, |state, cx| state.on_mouse_down(event, window, cx));
                }
            })
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_up(
                MouseButton::Right,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_move(window.listener_for(&self.state, InputState::on_mouse_move))
            .on_scroll_wheel(window.listener_for(&self.state, InputState::on_scroll_wheel))
            .size_full()
            .line_height(LINE_HEIGHT)
            .input_px(self.size)
            .input_py(self.size)
            .input_h(self.size)
            .input_text_size(self.size)
            .when(!self.disabled, |this| this.cursor_text())
            .items_center()
            .when(state.mode.is_multi_line(), |this| {
                this.h_auto()
                    .when_some(self.height, |this, height| this.h(height))
            })
            .when(self.appearance, |this| {
                this.bg(bg)
                    .rounded(cx.theme().radius)
                    .when(self.bordered, |this| {
                        this.border_color(border_color)
                            .border_1()
                            .when(cx.theme().shadow, |this| this.shadow_xs())
                            .when(focused && self.focus_bordered, |this| {
                                this.focused_border(cx)
                            })
                    })
            })
            .items_center()
            .gap(gap_x)
            .refine_style(&self.style)
            .children(prefix.map(|p| {
                div()
                    .when(state.disabled, |this| this.opacity(0.5))
                    .child(p)
            }))
            .when(state.mode.is_multi_line(), |mut this| {
                let paddings = this.style().padding.clone();
                this.child(Self::render_editor(paddings, &self.state, &state, window))
            })
            .when(!state.mode.is_multi_line(), |this| {
                this.child(self.state.clone())
            })
            .when(has_suffix, |this| {
                this.pr(self.size.input_px()).child(
                    h_flex()
                        .id("suffix")
                        .gap(gap_x)
                        .items_center()
                        .when(state.disabled, |this| this.opacity(0.5))
                        .when(state.loading, |this| {
                            this.child(Spinner::new().color(cx.theme().muted_foreground))
                        })
                        .when(self.mask_toggle, |this| {
                            this.child(Self::render_toggle_mask_button(&self.state, cx))
                        })
                        .when(show_clear_button, |this| {
                            this.child(clear_button(cx).on_click({
                                let state = self.state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.clean(window, cx);
                                        state.focus(window, cx);
                                    })
                                }
                            }))
                        })
                        .children(suffix),
                )
            })
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn content_type_maps_to_ns_text_content_type_values() {
        let content_types = [
            (InputContentType::Name, Some("name")),
            (InputContentType::NamePrefix, Some("honorific-prefix")),
            (InputContentType::GivenName, Some("given-name")),
            (InputContentType::MiddleName, Some("additional-name")),
            (InputContentType::FamilyName, Some("family-name")),
            (InputContentType::NameSuffix, Some("honorific-suffix")),
            (InputContentType::Nickname, Some("nickname")),
            (InputContentType::JobTitle, Some("organization-title")),
            (InputContentType::OrganizationName, Some("organization")),
            (InputContentType::Location, Some("location")),
            (InputContentType::FullStreetAddress, Some("street-address")),
            (InputContentType::StreetAddressLine1, Some("address-line1")),
            (InputContentType::StreetAddressLine2, Some("address-line2")),
            (InputContentType::AddressCity, Some("address-level2")),
            (InputContentType::AddressState, Some("address-level1")),
            (
                InputContentType::AddressCityAndState,
                Some("address-level1+2"),
            ),
            (InputContentType::Sublocality, Some("address-level3")),
            (InputContentType::CountryName, Some("country-name")),
            (InputContentType::PostalCode, Some("postal-code")),
            (InputContentType::TelephoneNumber, Some("tel")),
            (InputContentType::EmailAddress, Some("email")),
            (InputContentType::Url, Some("url")),
            (InputContentType::CreditCardNumber, Some("cc-number")),
            (InputContentType::CreditCardName, Some("cc-name")),
            (InputContentType::CreditCardGivenName, Some("cc-given-name")),
            (
                InputContentType::CreditCardMiddleName,
                Some("cc-additional-name"),
            ),
            (
                InputContentType::CreditCardFamilyName,
                Some("cc-family-name"),
            ),
            (InputContentType::CreditCardSecurityCode, Some("cc-csc")),
            (InputContentType::CreditCardExpiration, Some("cc-exp")),
            (
                InputContentType::CreditCardExpirationMonth,
                Some("cc-exp-month"),
            ),
            (
                InputContentType::CreditCardExpirationYear,
                Some("cc-exp-year"),
            ),
            (InputContentType::CreditCardType, Some("cc-type")),
            (InputContentType::Username, Some("username")),
            (InputContentType::Password, Some("password")),
            (InputContentType::NewPassword, Some("new-password")),
            (InputContentType::OneTimeCode, Some("one-time-code")),
            (
                InputContentType::ShipmentTrackingNumber,
                Some("shipment-tracking-number"),
            ),
            (InputContentType::FlightNumber, Some("flight-number")),
            (InputContentType::DateTime, Some("date-time")),
            (InputContentType::Birthdate, Some("bday")),
            (InputContentType::BirthdateDay, Some("bday-day")),
            (InputContentType::BirthdateMonth, Some("bday-month")),
            (InputContentType::BirthdateYear, Some("bday-year")),
            (InputContentType::CellularEid, None),
            (InputContentType::CellularImei, None),
        ];

        for (content_type, native_value) in content_types {
            assert_eq!(content_type.ns_text_content_type(), native_value);
        }
    }
}
