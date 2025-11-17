use std::{any::Any, rc::Rc};

use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, ClickEvent, ElementId, Empty,
    InteractiveElement as _, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::{checkbox::Checkbox, h_flex, label::Label, switch::Switch, v_flex, ActiveTheme as _};

/// The type of setting field to render.
#[derive(Clone, Copy)]
pub enum SettingFieldType {
    Switch,
    Checkbox,
    NumberInput,
    Input,
    Dropdown,
}

/// A setting field that can get and set a value of type T in the App.
pub struct SettingField<T> {
    pub value: fn(&App) -> T,
    pub set_value: fn(T, &mut App),
    pub reset_value: fn(&mut App),
}

pub trait AnySettingField {
    fn as_any(&self) -> &dyn std::any::Any;
    fn type_name(&self) -> &'static str;
    fn type_id(&self) -> std::any::TypeId;
    fn reset_value(&self, cx: &mut App);
}

impl<T: Clone + PartialEq + Send + Sync + 'static> AnySettingField for SettingField<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<T>()
    }

    fn reset_value(&self, cx: &mut App) {
        (self.reset_value)(cx);
    }
}

#[derive(Clone)]
pub enum SettingItem {
    Item {
        id: &'static str,
        label: SharedString,
        description: Option<SharedString>,
        field_type: SettingFieldType,
        field: Rc<dyn AnySettingField + Send + Sync>,
    },
    Element {
        id: &'static str,
        element: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>,
    },
}

impl SettingItem {
    pub(crate) fn is_default(&self) -> bool {
        match self {
            SettingItem::Item { .. } => false,
            SettingItem::Element { .. } => true,
        }
    }

    pub(crate) fn is_match(&self, query: &str) -> bool {
        match self {
            SettingItem::Item {
                label, description, ..
            } => {
                label.to_lowercase().contains(&query.to_lowercase())
                    || description
                        .as_ref()
                        .map_or(false, |d| d.to_lowercase().contains(&query.to_lowercase()))
            }
            SettingItem::Element { .. } => false,
        }
    }

    // pub(crate) fn on_reset(&self) -> Rc<impl Fn(&mut App)> {
    //     match self {
    //         SettingItem::Item { field, .. } => {
    //             let field = field.clone();
    //             let reset_value = Rc::new(|cx: &mut App| {
    //                 field.reset_value(cx);
    //             });
    //             reset_value
    //         }
    //         SettingItem::Element { .. } => Rc::new(|_: &mut App| {}),
    //     }
    // }

    fn render_field(
        id: &'static str,
        label: SharedString,
        description: Option<SharedString>,
        field_type: SettingFieldType,
        field: Rc<dyn AnySettingField>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let type_name = field.type_name();
        match field.type_id() {
            t if t == std::any::TypeId::of::<bool>() => {
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

                if matches!(field_type, SettingFieldType::Checkbox) {
                    return Checkbox::new("check")
                        .checked(checked)
                        .on_click(move |checked: &bool, _, cx: &mut App| {
                            set_value(*checked, cx);
                        })
                        .into_any_element();
                } else {
                    Switch::new("check")
                        .checked(checked)
                        .on_click(move |checked: &bool, _, cx: &mut App| {
                            set_value(*checked, cx);
                        })
                        .into_any_element()
                }
            }
            _ => div()
                .child(Label::new(format!("Unsupported field type: {}", type_name)))
                .into_any_element(),
        }
    }

    pub(super) fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            SettingItem::Item {
                id,
                label,
                description,
                field_type,
                field,
            } => h_flex()
                .id(id)
                .gap_4()
                .justify_between()
                .child(
                    v_flex()
                        .gap_1()
                        .child(h_flex().justify_between().items_center().child(
                            v_flex().child(Label::new(label.clone())).when_some(
                                description.clone(),
                                |this, description| {
                                    this.child(
                                        Label::new(description)
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                },
                            ),
                        )),
                )
                .child(Self::render_field(
                    id,
                    label,
                    description,
                    field_type,
                    field,
                    window,
                    cx,
                )),
            SettingItem::Element { id, element } => div().id(id).child((element)(window, cx)),
        }
    }
}
