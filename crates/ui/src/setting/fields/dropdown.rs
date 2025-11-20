use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, AnyElement, App, Corner, IntoElement, SharedString,
    StyleRefinement, Styled, Window,
};

use crate::{
    button::{Button, ButtonVariants},
    group_box::GroupBoxVariant,
    menu::{DropdownMenu, PopupMenuItem},
    setting::{
        fields::{get_value, set_value, SettingFieldRender},
        AnySettingField, RenderOptions,
    },
    AxisExt, Sizable, StyledExt,
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
            .map(|this| match options.group_variant {
                GroupBoxVariant::Fill => this.ghost(),
                GroupBoxVariant::Outline => this.outline(),
                _ => this,
            })
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
