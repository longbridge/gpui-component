use crate::{
    checkbox::Checkbox,
    setting::fields::{get_value, set_value, SettingFieldRender},
    switch::Switch,
};
use gpui::{AnyElement, App, IntoElement};

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
