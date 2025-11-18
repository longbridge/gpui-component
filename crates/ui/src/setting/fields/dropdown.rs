use gpui::{
    div, AnyElement, App, AppContext as _, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, Styled, Window,
};

use crate::{
    button::{Button, DropdownButton},
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField, SettingFieldType,
    },
};

pub(crate) struct DropdownField<T> {
    options: Vec<(SharedString, SharedString)>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> DropdownField<T> {
    pub(crate) fn new(options: Option<&Vec<(SharedString, SharedString)>>) -> Self {
        Self {
            options: options.cloned().unwrap_or(vec![]),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> SettingFieldRender for DropdownField<T>
where
    T: Into<SharedString> + From<SharedString> + Clone + 'static,
{
    fn render(
        &self,
        id: &'static str,
        _label: SharedString,
        _description: Option<SharedString>,
        field: std::rc::Rc<dyn AnySettingField>,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let old_value = get_value::<T>(&field, cx);
        let set_value = set_value::<T>(&field, cx);
        let options = self.options.clone();

        let old_label = options
            .iter()
            .find(|(value, _)| *value == old_value.clone().into())
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| old_value.clone().into());

        div()
            .id(id)
            .w_64()
            .child(
                DropdownButton::new("btn")
                    .label(old_label)
                    .outline()
                    .dropdown_menu(move |menu, _, _| {
                        let menu = options.iter().fold(menu, |menu, (value, label)| {
                            let old_value: SharedString = old_value.clone().into();
                            let checked = &old_value == value;
                            menu.item(PopupMenuItem::new(label.clone()).checked(checked).on_click(
                                {
                                    let value = value.clone();
                                    move |_, _, cx| {
                                        set_value(T::from(value.clone()), cx);
                                    }
                                },
                            ))
                        });

                        menu
                    }),
            )
            .into_any_element()
    }
}
