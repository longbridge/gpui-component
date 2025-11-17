use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement as _, SharedString, Window};

use crate::{
    label::Label,
    setting::{fields::SettingFieldRender, AnySettingField},
};

pub(crate) struct UnknownField;

impl SettingFieldRender for UnknownField {
    fn render(
        &self,
        _: &'static str,
        _: SharedString,
        _: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        _: &mut Window,
        _: &mut App,
    ) -> AnyElement {
        let type_name = field.type_name();
        Label::new(format!("Unsupported: {}", type_name)).into_any_element()
    }
}
