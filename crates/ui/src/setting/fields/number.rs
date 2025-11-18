use gpui::{
    div, AnyElement, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Styled,
};

use crate::{
    input::{InputEvent, InputState, NumberInput},
    setting::fields::{get_value, set_value, SettingFieldRender},
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
        _label: gpui::SharedString,
        _description: Option<gpui::SharedString>,
        field: std::rc::Rc<dyn crate::setting::AnySettingField>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> AnyElement {
        let value = get_value::<f64>(&field, cx);
        let set_value = set_value::<f64>(&field, cx);

        let state = window
            .use_keyed_state(id, cx, |window, cx| {
                let state =
                    cx.new(|cx| InputState::new(window, cx).default_value(value.to_string()));
                let subscription = cx.subscribe(&state, {
                    move |_, state, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            if let Ok(value) = state.read(cx).value().parse::<f64>() {
                                set_value(value, cx);
                            }
                        }
                        _ => return,
                    }
                });

                State {
                    input: state,
                    _subscription: subscription,
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
