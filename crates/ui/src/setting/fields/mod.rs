mod bool;
mod dropdown;
mod number;
mod string;

pub(crate) use bool::*;
pub(crate) use dropdown::*;
pub(crate) use number::*;
pub(crate) use string::*;

use gpui::{AnyElement, App, SharedString, Window};
use std::rc::Rc;

use crate::setting::{AnySettingField, SettingField};

pub(crate) trait SettingFieldRender {
    #[allow(unused)]
    fn render(
        &self,
        id: &'static str,
        label: SharedString,
        description: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;
}

pub(crate) fn get_value<T: Clone + 'static>(field: &Rc<dyn AnySettingField>, cx: &mut App) -> T {
    let setting_field = field
        .as_any()
        .downcast_ref::<SettingField<T>>()
        .expect("Failed to downcast setting field");
    (setting_field.value)(cx)
}

pub(crate) fn set_value<T: Clone + 'static>(
    field: &Rc<dyn AnySettingField>,
    _cx: &mut App,
) -> fn(T, &mut App) {
    let setting_field = field
        .as_any()
        .downcast_ref::<SettingField<T>>()
        .expect("Failed to downcast setting field");
    setting_field.set_value
}
