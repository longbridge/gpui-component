use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, AnyElement, App, AppContext as _, Axis, Entity, IntoElement,
    SharedString, StyleRefinement, Styled, Window,
};

use crate::{
    input::{Input, InputEvent, InputState},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField,
    },
    AxisExt as _, Sizable, Size, StyledExt,
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
        field: Rc<dyn AnySettingField>,
        size: Size,
        layout: Axis,
        style: &StyleRefinement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let value = get_value::<T>(&field, cx);
        let set_value = set_value::<T>(&field, cx);

        let state = window
            .use_keyed_state("string-state", cx, |window, cx| {
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

        Input::new(&state.input)
            .with_size(size)
            .map(|this| {
                if layout.is_horizontal() {
                    this.w_64()
                } else {
                    this.flex_1().min_w_64().w_full()
                }
            })
            .refine_style(style)
            .into_any_element()
    }
}
