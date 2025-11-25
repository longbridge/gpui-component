use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, AnyElement, App, AppContext as _, Entity, IntoElement,
    SharedString, StyleRefinement, Styled, Window,
};

use crate::{
    input::{InputState, NumberInput, NumberInputEvent},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField, RenderOptions,
    },
    AxisExt, Sizable, StyledExt,
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
        field: Rc<dyn AnySettingField>,
        options: &RenderOptions,
        style: &StyleRefinement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let value = get_value::<f64>(&field, cx);
        let set_value = set_value::<f64>(&field, cx);
        let num_options = self.options.clone();

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
                                    (value + num_options.step).min(num_options.max)
                                } else {
                                    (value - num_options.step).max(num_options.min)
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

        NumberInput::new(&state.input)
            .with_size(options.size)
            .map(|this| {
                if options.layout.is_horizontal() {
                    this.w_32()
                } else {
                    this.w_full()
                }
            })
            .refine_style(style)
            .into_any_element()
    }
}
