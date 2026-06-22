use std::rc::Rc;

use crate::theme::ActiveTheme;
use gpui::Corners;
use gpui::Window;
use gpui::{AnyElement, App, Context, Edges, Entity, EventEmitter, FocusHandle, Focusable};
use gpui::{
    InteractiveElement, IntoElement, KeyBinding, ParentElement, RenderOnce, SharedString,
    StyleRefinement, Styled, TextAlign, actions, prelude::FluentBuilder as _,
};

use crate::{
    Disableable, IconName, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
};

use super::{Input, InputState, MaskPattern};

actions!(number_input, [Increment, Decrement]);

const CONTEXT: &str = "NumberInput";
pub fn init(cx: &mut App) {
    cx.bind_keys(vec![
        KeyBinding::new("up", Increment, Some(CONTEXT)),
        KeyBinding::new("down", Decrement, Some(CONTEXT)),
    ]);
}

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
            state.on_action_increment(&Increment, window, cx);
        })
    }

    fn on_decrement(state: &Entity<InputState>, window: &mut Window, cx: &mut App) {
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            state.on_action_decrement(&Decrement, window, cx);
        })
    }
}

impl Disableable for NumberInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl InputState {
    fn on_action_increment(&mut self, _: &Increment, window: &mut Window, cx: &mut Context<Self>) {
        self.on_number_input_step(StepAction::Increment, window, cx);
    }

    fn on_action_decrement(&mut self, _: &Decrement, window: &mut Window, cx: &mut Context<Self>) {
        self.on_number_input_step(StepAction::Decrement, window, cx);
    }

    fn on_number_input_step(
        &mut self,
        action: StepAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }

        // By default NumberInput steps the value internally with step 1.
        // To opt out and emit `NumberInputEvent::Step` instead (the caller
        // updates the value), call `state.set_step(None, window, cx)`.
        if let Some(step) = self.number_step.clone() {
            let value = self.unmask_value();
            let current = value.trim().parse::<f64>().unwrap_or(0.);
            let step = step.value(current, action, cx);
            if let Some(new_value) =
                step_value(&value, action, step, self.number_min, self.number_max)
            {
                // The stepped value must pass the `pattern`/`validate` check,
                // otherwise fall back to emit the event to let the caller handle it.
                if self.is_valid_input(&new_value, cx) {
                    let range = self.range_to_utf16(&(0..self.text.len()));
                    self.replace_text_in_range_silent(Some(range), &new_value, window, cx);
                    return;
                }
            } else {
                // Stepping cannot move the value in this direction (e.g.
                // Decrement on a below-min value), do nothing.
                return;
            }
        }

        cx.emit(NumberInputEvent::Step(action));
    }
}

/// The step strategy of the [`NumberInput`] for increment/decrement.
///
/// See also [`InputState::step`] and [`InputState::step_by`].
#[derive(Clone)]
pub enum NumberStep {
    /// A fixed step value.
    Fixed(f64),
    /// Calculate the step value from the current value and direction.
    ByValue(Rc<dyn Fn(f64, StepAction, &mut Context<InputState>) -> f64>),
}

impl NumberStep {
    /// Create a step that calculates the step value from the current value
    /// and direction on stepping.
    ///
    /// The current value is the value before stepping; an empty or invalid
    /// value is treated as 0. The [`StepAction`] tells whether the value is
    /// being incremented or decremented, useful when the step differs by
    /// direction at a range boundary.
    ///
    /// The closure receives a [`Context<InputState>`] to read or update other
    /// entities while computing the step, but must not re-enter the owning
    /// [`InputState`] (it is mutably borrowed during stepping).
    pub fn by_value(
        f: impl Fn(f64, StepAction, &mut Context<InputState>) -> f64 + 'static,
    ) -> Self {
        Self::ByValue(Rc::new(f))
    }

    /// Return the step value for the given current value and direction.
    pub(super) fn value(
        &self,
        current: f64,
        action: StepAction,
        cx: &mut Context<InputState>,
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

/// Step the `value` by `step` and clamp the result to the `min`/`max` range.
///
/// Returns `None` if stepping cannot move the value in the given direction
/// (e.g. the value is already at the boundary).
///
/// The result keeps the max fraction digits of the current value and the step,
/// to avoid float precision issue, e.g. `0.1 + 0.2 -> 0.3`.
fn step_value(
    value: &str,
    action: StepAction,
    step: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> Option<String> {
    fn fraction_digits(value: &str) -> usize {
        value.split('.').nth(1).map_or(0, |frac| frac.len())
    }

    let current = value.trim().parse::<f64>().ok();
    let mut new_value = match action {
        StepAction::Increment => current.unwrap_or(0.) + step,
        StepAction::Decrement => current.unwrap_or(0.) - step,
    };
    let mut digits = fraction_digits(value).max(fraction_digits(&step.to_string()));
    if let Some(min) = min {
        if new_value < min {
            new_value = min;
            digits = digits.max(fraction_digits(&min.to_string()));
        }
    }
    if let Some(max) = max {
        if new_value > max {
            new_value = max;
            digits = digits.max(fraction_digits(&max.to_string()));
        }
    }

    // Web behavior: stepping must move the value in the pressed direction, so
    // a Decrement below min does nothing rather than clamping up. An empty or
    // invalid value always steps into the range.
    if let Some(current) = current {
        let moved = match action {
            StepAction::Increment => new_value > current,
            StepAction::Decrement => new_value < current,
        };
        if !moved {
            return None;
        }
    }

    Some(format!("{:.*}", digits, new_value))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StepAction {
    Decrement,
    Increment,
}
pub enum NumberInputEvent {
    Step(StepAction),
}
impl EventEmitter<NumberInputEvent> for InputState {}

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
        if !self.state.read(cx).mask_pattern_set {
            self.state.update(cx, |state, _| {
                state.mask_pattern = MaskPattern::Number {
                    separator: None,
                    fraction: None,
                };
            });
        }

        h_flex()
            .id(("number-input", self.state.entity_id()))
            .key_context(CONTEXT)
            .on_action(window.listener_for(&self.state, InputState::on_action_increment))
            .on_action(window.listener_for(&self.state, InputState::on_action_decrement))
            .flex_1()
            .rounded(cx.theme().radius)
            .refine_style(&self.style)
            .when(self.disabled, |this| this.opacity(0.5))
            .child(
                Button::new("minus")
                    .map(|this| {
                        if self.appearance {
                            this.outline()
                        } else {
                            this.ghost()
                        }
                    })
                    .with_size(self.size)
                    .icon(IconName::Minus)
                    .compact()
                    .tab_stop(false)
                    .disabled(self.disabled)
                    .border_color(cx.theme().input)
                    .border_corners(Corners {
                        top_left: true,
                        top_right: false,
                        bottom_right: false,
                        bottom_left: true,
                    })
                    .border_edges(Edges {
                        top: self.appearance,
                        right: false,
                        bottom: self.appearance,
                        left: self.appearance,
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
                    .appearance(self.appearance)
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
                    .map(|this| {
                        if self.appearance {
                            this.outline()
                        } else {
                            this.ghost()
                        }
                    })
                    .with_size(self.size)
                    .icon(IconName::Plus)
                    .compact()
                    .tab_stop(false)
                    .disabled(self.disabled)
                    .border_color(cx.theme().input)
                    .border_corners(Corners {
                        top_left: false,
                        top_right: true,
                        bottom_right: true,
                        bottom_left: false,
                    })
                    .border_edges(Edges {
                        top: self.appearance,
                        right: self.appearance,
                        bottom: self.appearance,
                        left: false,
                    })
                    .on_click({
                        let state = self.state.clone();
                        move |_, window, cx| {
                            Self::on_increment(&state, window, cx);
                        }
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{StepAction, step_value};

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
