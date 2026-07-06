use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, DefiniteLength, Edges, EdgesRefinement, Entity, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, MouseDownEvent, ParentElement as _, Rems, RenderOnce, Role,
    StatefulInteractiveElement as _, StyleRefinement, Styled, TextAlign, Window, div, px, relative,
};

use crate::button::{Button, ButtonVariants as _};
use crate::input::clear_button;
use crate::native_menu::NativeMenu;
use crate::spinner::Spinner;
use crate::{ActiveTheme, Colorize, v_flex};
use crate::{IconName, Size};
use crate::{Selectable, StyledExt, h_flex};
use crate::{Sizable, StyleSized};

use super::{
    InputContentType, InputState, content_type::sync_native_content_type, element::EditorScrollbar,
};

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
    role: Option<Role>,

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
            role: None,
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

    /// Override the accessible role for the input.
    ///
    /// If unset, the role is inferred from multi-line mode and content type.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
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

    fn mouse_down_handler(
        state: Entity<InputState>,
        content_type: Option<InputContentType>,
        disabled: bool,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        move |event, window, cx| {
            sync_native_content_type(window, content_type, disabled);
            state.update(cx, |state, cx| state.on_mouse_down(event, window, cx));
        }
    }

    fn accessibility_role(
        is_multi_line: bool,
        content_type: Option<InputContentType>,
        role: Option<Role>,
    ) -> Role {
        if let Some(role) = role {
            return role;
        }

        if is_multi_line {
            return Role::MultilineTextInput;
        }

        match content_type {
            None => Role::TextInput,
            Some(InputContentType::TelephoneNumber) => Role::PhoneNumberInput,
            Some(InputContentType::EmailAddress) => Role::EmailInput,
            Some(InputContentType::Url) => Role::UrlInput,
            Some(InputContentType::Password | InputContentType::NewPassword) => Role::PasswordInput,
            Some(InputContentType::DateTime) => Role::DateTimeInput,
            Some(InputContentType::Birthdate) => Role::DateInput,
            Some(
                InputContentType::Name
                | InputContentType::NamePrefix
                | InputContentType::GivenName
                | InputContentType::MiddleName
                | InputContentType::FamilyName
                | InputContentType::NameSuffix
                | InputContentType::Nickname
                | InputContentType::JobTitle
                | InputContentType::OrganizationName
                | InputContentType::Location
                | InputContentType::FullStreetAddress
                | InputContentType::StreetAddressLine1
                | InputContentType::StreetAddressLine2
                | InputContentType::AddressCity
                | InputContentType::AddressState
                | InputContentType::AddressCityAndState
                | InputContentType::Sublocality
                | InputContentType::CountryName
                | InputContentType::PostalCode
                | InputContentType::CreditCardNumber
                | InputContentType::CreditCardName
                | InputContentType::CreditCardGivenName
                | InputContentType::CreditCardMiddleName
                | InputContentType::CreditCardFamilyName
                | InputContentType::CreditCardSecurityCode
                | InputContentType::CreditCardExpiration
                | InputContentType::CreditCardExpirationMonth
                | InputContentType::CreditCardExpirationYear
                | InputContentType::CreditCardType
                | InputContentType::Username
                | InputContentType::OneTimeCode
                | InputContentType::ShipmentTrackingNumber
                | InputContentType::FlightNumber
                | InputContentType::BirthdateDay
                | InputContentType::BirthdateMonth
                | InputContentType::BirthdateYear
                | InputContentType::CellularEid
                | InputContentType::CellularImei,
            ) => Role::TextInput,
        }
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
        let is_multi_line = state.mode.is_multi_line();
        let accessibility_role = Self::accessibility_role(is_multi_line, content_type, self.role);
        let focused = state.focus_handle.is_focused(window) && !state.disabled;
        if focused {
            sync_native_content_type(window, content_type, state.disabled);
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
            .role(accessibility_role)
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
            .on_mouse_down(
                MouseButton::Left,
                Self::mouse_down_handler(self.state.clone(), content_type, disabled),
            )
            .on_mouse_down(
                MouseButton::Right,
                Self::mouse_down_handler(self.state.clone(), content_type, disabled),
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_types_map_to_accessibility_roles() {
        let cases = [
            (None, Role::TextInput),
            (Some(InputContentType::Name), Role::TextInput),
            (Some(InputContentType::NamePrefix), Role::TextInput),
            (Some(InputContentType::GivenName), Role::TextInput),
            (Some(InputContentType::MiddleName), Role::TextInput),
            (Some(InputContentType::FamilyName), Role::TextInput),
            (Some(InputContentType::NameSuffix), Role::TextInput),
            (Some(InputContentType::Nickname), Role::TextInput),
            (Some(InputContentType::JobTitle), Role::TextInput),
            (Some(InputContentType::OrganizationName), Role::TextInput),
            (Some(InputContentType::Location), Role::TextInput),
            (Some(InputContentType::FullStreetAddress), Role::TextInput),
            (Some(InputContentType::StreetAddressLine1), Role::TextInput),
            (Some(InputContentType::StreetAddressLine2), Role::TextInput),
            (Some(InputContentType::AddressCity), Role::TextInput),
            (Some(InputContentType::AddressState), Role::TextInput),
            (Some(InputContentType::AddressCityAndState), Role::TextInput),
            (Some(InputContentType::Sublocality), Role::TextInput),
            (Some(InputContentType::CountryName), Role::TextInput),
            (Some(InputContentType::PostalCode), Role::TextInput),
            (
                Some(InputContentType::TelephoneNumber),
                Role::PhoneNumberInput,
            ),
            (Some(InputContentType::EmailAddress), Role::EmailInput),
            (Some(InputContentType::Url), Role::UrlInput),
            (Some(InputContentType::CreditCardNumber), Role::TextInput),
            (Some(InputContentType::CreditCardName), Role::TextInput),
            (Some(InputContentType::CreditCardGivenName), Role::TextInput),
            (
                Some(InputContentType::CreditCardMiddleName),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardFamilyName),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardSecurityCode),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpiration),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpirationMonth),
                Role::TextInput,
            ),
            (
                Some(InputContentType::CreditCardExpirationYear),
                Role::TextInput,
            ),
            (Some(InputContentType::CreditCardType), Role::TextInput),
            (Some(InputContentType::Username), Role::TextInput),
            (Some(InputContentType::Password), Role::PasswordInput),
            (Some(InputContentType::NewPassword), Role::PasswordInput),
            (Some(InputContentType::OneTimeCode), Role::TextInput),
            (
                Some(InputContentType::ShipmentTrackingNumber),
                Role::TextInput,
            ),
            (Some(InputContentType::FlightNumber), Role::TextInput),
            (Some(InputContentType::DateTime), Role::DateTimeInput),
            (Some(InputContentType::Birthdate), Role::DateInput),
            (Some(InputContentType::BirthdateDay), Role::TextInput),
            (Some(InputContentType::BirthdateMonth), Role::TextInput),
            (Some(InputContentType::BirthdateYear), Role::TextInput),
            (Some(InputContentType::CellularEid), Role::TextInput),
            (Some(InputContentType::CellularImei), Role::TextInput),
        ];

        for (content_type, role) in cases {
            assert_eq!(Input::accessibility_role(false, content_type, None), role);
        }
    }

    #[test]
    fn multiline_inputs_keep_multiline_accessibility_role() {
        assert_eq!(
            Input::accessibility_role(true, Some(InputContentType::Password), None),
            Role::MultilineTextInput
        );
    }

    #[test]
    fn explicit_accessibility_role_overrides_defaults() {
        assert_eq!(
            Input::accessibility_role(
                false,
                Some(InputContentType::Password),
                Some(Role::TextInput)
            ),
            Role::TextInput
        );
        assert_eq!(
            Input::accessibility_role(
                true,
                Some(InputContentType::Password),
                Some(Role::TextInput)
            ),
            Role::TextInput
        );
    }
}
