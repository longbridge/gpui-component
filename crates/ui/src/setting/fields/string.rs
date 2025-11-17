use gpui::{
    div, AnyElement, AppContext as _, Entity, IntoElement, ParentElement as _, SharedString, Styled,
};

use crate::{
    input::{InputEvent, InputState, NumberInput},
    setting::{fields::SettingFieldRender, SettingField},
};

pub(crate) struct StringField<T: Into<SharedString>> {
    pub _marker: std::marker::PhantomData<T>,
}

struct State {
    input: Entity<InputState>,
    _subscription: gpui::Subscription,
}

impl<T: Into<SharedString>> SettingFieldRender for StringField<T> {
    fn render(
        &self,
        id: &'static str,
        _label: gpui::SharedString,
        _description: Option<gpui::SharedString>,
        field: std::rc::Rc<dyn crate::setting::AnySettingField>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> AnyElement {
        let value = (field
            .as_any()
            .downcast_ref::<SettingField<T>>()
            .unwrap()
            .value)(cx);
        let set_value = field
            .as_any()
            .downcast_ref::<SettingField<T>>()
            .unwrap()
            .set_value;

        let state = window
            .use_keyed_state(id, cx, |window, cx| {
                let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
                let subscription = cx.subscribe(&state, {
                    move |_, state, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            let value = state.read(cx).value().to_string();
                            set_value(value, cx);
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
            .w_32()
            .child(NumberInput::new(&state.input))
            .into_any_element()
    }
}
