use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, EventEmitter, InteractiveElement as _, IntoElement,
    KeyBinding, ParentElement, RenderOnce, Role, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, actions, prelude::FluentBuilder as _,
};

use crate::input::InputState;
pub use crate::input::NumberStep;
use crate::{Input, StyledExt as _};

actions!(number_input, [Increment, Decrement]);

const CONTEXT: &str = "NumberInput";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", Increment, Some(CONTEXT)),
        KeyBinding::new("down", Decrement, Some(CONTEXT)),
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepAction {
    Decrement,
    Increment,
}

pub enum NumberInputEvent {
    Step(StepAction),
}
impl EventEmitter<NumberInputEvent> for InputState {}

type StepHandler = Rc<dyn Fn(StepAction, &mut Window, &mut App)>;

/// An unstyled spinbutton root composed from the foundational [`Input`] frame.
#[derive(IntoElement)]
pub struct NumberInput {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    appearance: bool,
    disabled: bool,
    focused: bool,
    value: Option<f64>,
    on_step: Option<StepHandler>,
}

impl NumberInput {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            appearance: true,
            disabled: false,
            focused: false,
            value: None,
            on_step: None,
        }
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn value(mut self, value: Option<f64>) -> Self {
        self.value = value;
        self
    }

    pub fn on_step(
        mut self,
        handler: impl Fn(StepAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }
}

impl NumberInput {
    /// Bind a number input directly to an editor state, including focus and step behavior.
    pub fn bind_state(mut self, state: &Entity<InputState>) -> Self {
        let state = state.clone();
        self.on_step = Some(Rc::new(move |action, window, cx| {
            state.update(cx, |state, cx| {
                state.focus(window, cx);
                state.on_number_input_step(action, window, cx);
            });
        }));
        self
    }
}

impl Styled for NumberInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for NumberInput {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for NumberInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let on_step = self.on_step;

        Input::new(self.id)
            .appearance(self.appearance)
            .focused(self.focused && !disabled)
            .role(Role::SpinButton)
            .when_some(self.value, |this, value| this.aria_numeric_value(value))
            .key_context(CONTEXT)
            .on_action({
                let on_step = on_step.clone();
                move |_: &Increment, window, cx| {
                    if disabled {
                        cx.propagate();
                    } else if let Some(handler) = on_step.as_ref() {
                        handler(StepAction::Increment, window, cx);
                    }
                }
            })
            .on_action(move |_: &Decrement, window, cx| {
                if disabled {
                    cx.propagate();
                } else if let Some(handler) = on_step.as_ref() {
                    handler(StepAction::Decrement, window, cx);
                }
            })
            .children(self.children)
            .refine_style(&self.style)
            .render(window, cx)
    }
}

/// Step a numeric string while preserving decimal precision and range direction.
pub fn step_value(
    value: &str,
    action: StepAction,
    step: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> Option<String> {
    fn fraction_digits(value: &str) -> usize {
        value.split('.').nth(1).map_or(0, |fraction| fraction.len())
    }

    let current = value.trim().parse::<f64>().ok();
    let mut new_value = match action {
        StepAction::Increment => current.unwrap_or(0.) + step,
        StepAction::Decrement => current.unwrap_or(0.) - step,
    };
    let mut digits = fraction_digits(value).max(fraction_digits(&step.to_string()));
    if let Some(min) = min
        && new_value < min
    {
        new_value = min;
        digits = digits.max(fraction_digits(&min.to_string()));
    }
    if let Some(max) = max
        && new_value > max
    {
        new_value = max;
        digits = digits.max(fraction_digits(&max.to_string()));
    }

    if let Some(current) = current {
        let moved = match action {
            StepAction::Increment => new_value > current,
            StepAction::Decrement => new_value < current,
        };
        if !moved {
            return None;
        }
    }

    Some(format!("{new_value:.digits$}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_preserves_precision_and_directional_bounds() {
        assert_eq!(
            step_value("0.1", StepAction::Increment, 0.2, None, None).as_deref(),
            Some("0.3")
        );
        assert_eq!(
            step_value("10", StepAction::Decrement, 1., Some(10.), None),
            None
        );
        assert_eq!(
            step_value("99.5", StepAction::Increment, 1., None, Some(100.)).as_deref(),
            Some("100.0")
        );
        assert_eq!(
            step_value("10", StepAction::Increment, 1., None, Some(10.)),
            None
        );
        assert_eq!(
            step_value("5", StepAction::Decrement, 10., Some(0.), None).as_deref(),
            Some("0")
        );
    }
}
