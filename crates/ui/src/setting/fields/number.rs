use std::rc::Rc;

use gpui::{
    div, AnyElement, App, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, Styled, Window,
};

use crate::{
    input::{InputEvent, InputState, NumberInput},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField,
    },
};

pub(crate) struct NumberField;

struct State {
    input: Entity<InputState>,
    _subscription: gpui::Subscription,
}

impl SettingFieldRender for NumberField {
    fn render(
        &self,
        id: &'static str,
        _label: SharedString,
        _description: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let value = get_value::<f64>(&field, cx);
        let set_value = set_value::<f64>(&field, cx);

        let state = window
            .use_keyed_state(id, cx, |window, cx| {
                let input =
                    cx.new(|cx| InputState::new(window, cx).default_value(value.to_string()));
                let _subscription = cx.subscribe(&input, {
                    move |_, input, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            let value = input.read(cx).value();
                            dbg!(&value);
                            if let Ok(value) = value.parse::<f64>() {
                                set_value(value, cx);
                            }
                        }
                        _ => {}
                    }
                });

                State {
                    input,
                    _subscription,
                }
            })
            .read(cx);

        div()
            .id(id)
            .w_32()
            .child(NumberInput::new(&state.input))
            .into_any_element()
    }
}
