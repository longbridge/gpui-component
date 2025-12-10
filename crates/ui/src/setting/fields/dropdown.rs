use std::rc::Rc;

use gpui::{
    AnyElement, App, Corner, IntoElement, SharedString, StyleRefinement, Styled, Window,
    prelude::FluentBuilder as _,
};

use crate::{
    AxisExt, Sizable, StyledExt,
    button::Button,
    menu::{DropdownMenu, PopupMenuItem},
    setting::{
        AnySettingField, RenderOptions,
        fields::{SettingFieldRender, get_value, set_value},
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
        field: Rc<dyn AnySettingField>,
        options: &RenderOptions,
        style: &StyleRefinement,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let old_value = get_value::<T>(&field, cx);
        let set_value = set_value::<T>(&field, cx);
        let dropdown_options = self.options.clone();

        let old_label = dropdown_options
            .iter()
            .find(|(value, _)| *value == old_value.clone().into())
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| old_value.clone().into());

        Button::new("btn")
            .when(options.layout.is_vertical(), |this| this.w_full())
            .label(old_label)
            .dropdown_caret(true)
            .outline()
            .with_size(options.size)
            .refine_style(style)
            .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _, _| {
                let set_value = set_value.clone();
                let menu = dropdown_options.iter().fold(menu, |menu, (value, label)| {
                    let old_value: SharedString = old_value.clone().into();
                    let checked = &old_value == value;
                    menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(checked)
                            .on_click({
                                let value = value.clone();
                                let set_value = set_value.clone();
                                move |_, _, cx| {
                                    set_value(T::from(value.clone()), cx);
                                }
                            }),
                    )
                });
                menu
            })
            .into_any_element()
    }
}
