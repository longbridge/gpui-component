use std::rc::Rc;

use crate::{
    checkbox::Checkbox,
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField,
    },
    switch::Switch,
};
use gpui::{AnyElement, App, IntoElement, SharedString, Window};

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
        _label: SharedString,
        _description: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let checked = get_value::<bool>(&field, cx);
        let set_value = set_value::<bool>(&field, cx);

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
