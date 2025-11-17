use gpui::{div, AnyElement, App, IntoElement, ParentElement as _};

use crate::{
    checkbox::Checkbox,
    setting::{fields::SettingFieldRender, SettingField},
    switch::Switch,
};

pub(crate) struct BoolField {
    use_switch: bool,
}

impl BoolField {
    pub fn new(use_switch: bool) -> Self {
        Self { use_switch }
    }
}

impl SettingFieldRender for BoolField {
    fn render(
        &self,
        _: &'static str,
        _label: gpui::SharedString,
        _description: Option<gpui::SharedString>,
        field: std::rc::Rc<dyn crate::setting::AnySettingField>,
        _: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> AnyElement {
        let checked = (field
            .as_any()
            .downcast_ref::<SettingField<bool>>()
            .unwrap()
            .value)(cx);
        let set_value = field
            .as_any()
            .downcast_ref::<SettingField<bool>>()
            .unwrap()
            .set_value;

        if self.use_switch {
            Switch::new("check")
                .checked(checked)
                .on_click(move |checked: &bool, _, cx: &mut App| {
                    set_value(*checked, cx);
                })
                .into_any_element()
        } else {
            Checkbox::new("check")
                .checked(checked)
                .on_click(move |checked: &bool, _, cx: &mut App| {
                    set_value(*checked, cx);
                })
                .into_any_element()
        }
    }
}
