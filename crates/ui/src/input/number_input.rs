use crate::theme::ActiveTheme;
use gpui::Corners;
use gpui::Window;
use gpui::{AnyElement, App, Entity, FocusHandle, Focusable, px};
use gpui::{
    IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, TextAlign,
    prelude::FluentBuilder as _,
};

use crate::{
    Disableable, IconName, Sizable, Size, StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
};

use super::{Input, InputState, input::input_style};
use gpui_base::NumberInput as BaseNumberInput;
pub use gpui_base::{NumberInputEvent, NumberStep, StepAction};

/// A number input element with increment and decrement buttons.
#[derive(IntoElement)]
pub struct NumberInput {
    state: Entity<InputState>,
    placeholder: SharedString,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    appearance: bool,
    disabled: bool,
    style: StyleRefinement,
}

impl NumberInput {
    /// Create a new [`NumberInput`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            placeholder: SharedString::default(),
            prefix: None,
            suffix: None,
            appearance: true,
            disabled: false,
            style: StyleRefinement::default(),
        }
    }

    /// Set the placeholder text of the number input.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the prefix element of the number input.
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the number input.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set the appearance of the number input, if false will no border and background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    fn on_increment(state: &Entity<InputState>, window: &mut Window, cx: &mut App) {
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            state.on_number_input_step(StepAction::Increment, window, cx);
        })
    }

    fn on_decrement(state: &Entity<InputState>, window: &mut Window, cx: &mut App) {
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            state.on_number_input_step(StepAction::Decrement, window, cx);
        })
    }
}

impl Disableable for NumberInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Focusable for NumberInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Sizable for NumberInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for NumberInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NumberInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Default to use `MaskPattern::Number` to limit the input to a valid
        // number (optional leading sign, digits and a single dot), and to
        // normalize full-width number characters, e.g. `12。5` -> `12.5`.
        //
        // Only when the user has not set a `mask_pattern` explicitly, so that
        // `set_mask_pattern(MaskPattern::None)` can be used to opt out.
        self.state.update(cx, |state, _| state.ensure_number_mask());

        let numeric_value = self.state.read(cx).value().parse::<f64>().ok();
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window) && !self.disabled;
        let (bg, _) = input_style(self.disabled, cx);
        // Transparent like a ghost button, but tinted to the frame on hover.
        let button_variant = ButtonCustomVariant::new(cx)
            .foreground(cx.theme().secondary_foreground)
            .hover(cx.theme().input.opacity(0.4))
            .active(cx.theme().input.opacity(0.6));
        // The buttons sit inside the 1px frame, so their corners are a pixel
        // tighter than the frame's, or they paint over its inner curve.
        let button_radius = if self.appearance {
            (cx.theme().radius - px(1.)).max(px(0.))
        } else {
            cx.theme().radius
        };

        let step_state = self.state.clone();

        BaseNumberInput::new(("number-input", self.state.entity_id()))
            .value(numeric_value)
            .appearance(self.appearance)
            .disabled(self.disabled)
            .focused(focused)
            .bind_state(&step_state)
            .flex_1()
            .rounded(cx.theme().radius)
            // The buttons are ghost, so the frame around the whole control is
            // drawn here instead of by each of its parts.
            .when(self.appearance, |this| {
                this.bg(bg).when(focused, |this| this.focused_border(cx))
            })
            .refine_style(&self.style)
            .when(self.disabled, |this| this.opacity(0.5))
            .child(
                Button::new("minus")
                    .custom(button_variant)
                    .rounded(button_radius)
                    .with_size(self.size)
                    .icon(IconName::Minus)
                    .compact()
                    .tab_stop(false)
                    .disabled(self.disabled)
                    // Only the outer corners are rounded, to follow the frame.
                    .border_corners(Corners {
                        top_left: true,
                        top_right: false,
                        bottom_right: false,
                        bottom_left: true,
                    })
                    .on_click({
                        let state = self.state.clone();
                        move |_, window, cx| {
                            Self::on_decrement(&state, window, cx);
                        }
                    }),
            )
            .child(
                Input::new(&self.state)
                    .appearance(false)
                    .with_size(self.size)
                    .disabled(self.disabled)
                    .gap_0()
                    .rounded_none()
                    .text_align(TextAlign::Center)
                    .when_some(self.prefix, |this, prefix| this.prefix(prefix))
                    .when_some(self.suffix, |this, suffix| this.suffix(suffix)),
            )
            .child(
                Button::new("plus")
                    .custom(button_variant)
                    .rounded(button_radius)
                    .with_size(self.size)
                    .icon(IconName::Plus)
                    .compact()
                    .tab_stop(false)
                    .disabled(self.disabled)
                    .border_corners(Corners {
                        top_left: false,
                        top_right: true,
                        bottom_right: true,
                        bottom_left: false,
                    })
                    .on_click({
                        let state = self.state.clone();
                        move |_, window, cx| {
                            Self::on_increment(&state, window, cx);
                        }
                    }),
            )
            .render(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::StepAction;
    use gpui_base::step_value;

    // `test_number_step` lives in `state::tests` because `NumberStep::value`
    // now needs a `Context<InputState>` to invoke the `by_value` closure.

    #[test]
    fn test_step_value() {
        fn some(value: &str) -> Option<String> {
            Some(value.to_string())
        }

        // Step from empty value
        assert_eq!(
            step_value("", StepAction::Increment, 1., None, None),
            some("1")
        );
        assert_eq!(
            step_value("", StepAction::Decrement, 1., None, None),
            some("-1")
        );
        // Invalid intermediate values are treated as 0
        assert_eq!(
            step_value("-", StepAction::Increment, 1., None, None),
            some("1")
        );
        assert_eq!(
            step_value("1", StepAction::Increment, 1., None, None),
            some("2")
        );
        assert_eq!(
            step_value("-2", StepAction::Increment, 1., None, None),
            some("-1")
        );

        // Avoid float precision issue, e.g. 0.1 + 0.2 != 0.30000000000000004
        assert_eq!(
            step_value("0.1", StepAction::Increment, 0.2, None, None),
            some("0.3")
        );
        assert_eq!(
            step_value("0.3", StepAction::Decrement, 0.1, None, None),
            some("0.2")
        );
        // Keep the fraction digits of the current value
        assert_eq!(
            step_value("1.25", StepAction::Increment, 1., None, None),
            some("2.25")
        );

        // Step from empty value always steps into the range
        assert_eq!(
            step_value("", StepAction::Increment, 1., Some(10.), None),
            some("10")
        );
        assert_eq!(
            step_value("", StepAction::Decrement, 1., Some(10.), None),
            some("10")
        );
        // Clamp to min/max
        assert_eq!(
            step_value("99.5", StepAction::Increment, 1., None, Some(100.)),
            some("100.0")
        );
        assert_eq!(
            step_value("1000", StepAction::Decrement, 1., None, Some(100.)),
            some("100")
        );
        // Keep the fraction digits of the clamped bound
        assert_eq!(
            step_value("1", StepAction::Decrement, 1., Some(0.25), None),
            some("0.25")
        );

        // Stepping must move the value in the pressed direction:
        // no-op at the boundary
        assert_eq!(
            step_value("10", StepAction::Decrement, 1., Some(10.), None),
            None
        );
        assert_eq!(
            step_value("100", StepAction::Increment, 1., None, Some(100.)),
            None
        );
        // Decrement on a below-min value (or Increment on an above-max value)
        // does nothing, instead of moving the value in the opposite direction
        assert_eq!(
            step_value("5", StepAction::Decrement, 1., Some(10.), None),
            None
        );
        assert_eq!(
            step_value("1000", StepAction::Increment, 1., None, Some(100.)),
            None
        );
    }
}
