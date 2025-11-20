use std::rc::Rc;

use crate::{
    checkbox::Checkbox,
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField,
    },
    switch::Switch,
    Sizable, Size, StyledExt,
};
use gpui::{div, AnyElement, App, Axis, IntoElement, ParentElement as _, StyleRefinement, Window};

pub(crate) struct BoolField {
    use_switch: bool,
}

impl BoolField {
    pub(crate) fn new(use_switch: bool) -> Self {
        Self { use_switch }
    }
}

impl SettingFieldRender for BoolField {
    fn render(
        &self,
        field: Rc<dyn AnySettingField>,
        size: Size,
        _layout: Axis,
        style: &StyleRefinement,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let checked = get_value::<bool>(&field, cx);
        let set_value = set_value::<bool>(&field, cx);

        div()
            .refine_style(style)
            .child(if self.use_switch {
                Switch::new("check")
                    .checked(checked)
                    .with_size(size)
                    .on_click(move |checked: &bool, _, cx: &mut App| {
                        set_value(*checked, cx);
                    })
                    .into_any_element()
            } else {
                Checkbox::new("check")
                    .checked(checked)
                    .with_size(size)
                    .on_click(move |checked: &bool, _, cx: &mut App| {
                        set_value(*checked, cx);
                    })
                    .into_any_element()
            })
            .into_any_element()
    }
}
