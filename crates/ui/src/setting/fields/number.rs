use std::rc::Rc;

use gpui::{
    div, AnyElement, App, AppContext as _, Entity, IntoElement, ParentElement as _, SharedString,
    Styled, Window,
};

use crate::{
    input::{InputState, NumberInput, NumberInputEvent},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField,
    },
    Sizable, Size,
};

#[derive(Clone, Debug)]
pub struct NumberFieldOptions {
    /// The minimum value for the number input, default is `f64::MIN`.
    pub min: f64,
    /// The maximum value for the number input, default is `f64::MAX`.
    pub max: f64,
    /// The step value for the number input, default is `1.0`.
    pub step: f64,
}

impl Default for NumberFieldOptions {
    fn default() -> Self {
        Self {
            min: f64::MIN,
            max: f64::MAX,
            step: 1.0,
        }
    }
}

pub(crate) struct NumberField {
    options: NumberFieldOptions,
}

impl NumberField {
    pub(crate) fn new(options: Option<&NumberFieldOptions>) -> Self {
        Self {
            options: options.cloned().unwrap_or_default(),
        }
    }
}

struct State {
    input: Entity<InputState>,
    _subscription: gpui::Subscription,
}

impl SettingFieldRender for NumberField {
    fn render(
        &self,
        _label: SharedString,
        _description: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        size: Size,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let value = get_value::<f64>(&field, cx);
        let set_value = set_value::<f64>(&field, cx);
        let options = self.options.clone();

        let state = window
            .use_keyed_state("number-state", cx, |window, cx| {
                let input =
                    cx.new(|cx| InputState::new(window, cx).default_value(value.to_string()));
                let _subscription = cx.subscribe_in(&input, window, {
                    move |_, input, event: &NumberInputEvent, window, cx| match event {
                        NumberInputEvent::Step(action) => input.update(cx, |input, cx| {
                            let value = input.value();
                            if let Ok(value) = value.parse::<f64>() {
                                let new_value = if *action == crate::input::StepAction::Increment {
                                    (value + options.step).min(options.max)
                                } else {
                                    (value - options.step).max(options.min)
                                };
                                set_value(new_value, cx);
                                input.set_value(
                                    SharedString::from(new_value.to_string()),
                                    window,
                                    cx,
                                );
                            }
                        }),
                    }
                });

                State {
                    input,
                    _subscription,
                }
            })
            .read(cx);

        div()
            .w_32()
            .child(NumberInput::new(&state.input).with_size(size))
            .into_any_element()
    }
}
