use std::rc::Rc;

use gpui::{
    div, AnyElement, App, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, Styled, Window,
};

use crate::{
    input::{Input, InputEvent, InputState},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField,
    },
};

pub(crate) struct StringField<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> StringField<T> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

struct State {
    input: Entity<InputState>,
    _subscription: gpui::Subscription,
}

impl<T> SettingFieldRender for StringField<T>
where
    T: Into<SharedString> + From<SharedString> + Clone + 'static,
{
    fn render(
        &self,
        id: &'static str,
        _label: SharedString,
        _description: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let value = get_value::<T>(&field, cx);
        let set_value = set_value::<T>(&field, cx);

        let state = window
            .use_keyed_state(id, cx, |window, cx| {
                let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
                let _subscription = cx.subscribe(&input, {
                    move |_, input, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            let value = input.read(cx).value();
                            set_value(value.into(), cx);
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

        // TODO: Support width from field options.

        div()
            .id(id)
            .w_64()
            .child(Input::new(&state.input))
            .into_any_element()
    }
}
