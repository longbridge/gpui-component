use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, InteractiveElement as _, IntoElement,
    ParentElement, SharedString, Styled, Window,
};
use std::{
    any::{Any, TypeId},
    ops::Deref,
    rc::Rc,
};

use crate::{
    h_flex,
    label::Label,
    setting::fields::{
        BoolField, DropdownField, NumberField, NumberFieldOptions, SettingFieldRender, StringField,
    },
    v_flex, ActiveTheme as _,
};

/// The type of setting field to render.
#[derive(Clone, Debug)]
pub enum SettingFieldType {
    /// As switch toggle, required `bool` value.
    Switch,
    /// As checkbox, required `bool` value.
    Checkbox,
    /// As a number input, required `f64` value.
    NumberInput {
        options: NumberFieldOptions,
    },
    Input,
    Dropdown {
        /// The options for the dropdown as (value, label) pairs.
        options: Vec<(SharedString, SharedString)>,
    },
}

impl SettingFieldType {
    #[inline]
    pub fn is_switch(&self) -> bool {
        matches!(self, SettingFieldType::Switch)
    }

    #[inline]
    pub fn is_checkbox(&self) -> bool {
        matches!(self, SettingFieldType::Checkbox)
    }

    #[inline]
    pub fn is_number_input(&self) -> bool {
        matches!(self, SettingFieldType::NumberInput { .. })
    }

    #[inline]
    pub fn is_input(&self) -> bool {
        matches!(self, SettingFieldType::Input)
    }

    #[inline]
    pub fn is_dropdown(&self) -> bool {
        matches!(self, SettingFieldType::Dropdown { .. })
    }

    #[inline]
    pub(super) fn dropdown_options(&self) -> Option<&Vec<(SharedString, SharedString)>> {
        match self {
            SettingFieldType::Dropdown { options } => Some(options),
            _ => None,
        }
    }

    #[inline]
    pub fn number_input_options(&self) -> Option<&NumberFieldOptions> {
        match self {
            SettingFieldType::NumberInput { options } => Some(options),
            _ => None,
        }
    }
}

/// A setting field that can get and set a value of type T in the App.
pub struct SettingField<T> {
    /// Function to get the value for this field.
    pub(crate) value: Rc<dyn Fn(&App) -> T>,
    /// Function to set the value for this field.
    pub(crate) set_value: Rc<dyn Fn(T, &mut App)>,
    pub(crate) default_value: Option<T>,
}

impl<T> SettingField<T> {
    /// Create a new setting field with the given get and set functions.
    pub fn new<V, S>(value: V, set_value: S) -> Self
    where
        V: Fn(&App) -> T + 'static,
        S: Fn(T, &mut App) + 'static,
    {
        Self {
            value: Rc::new(value),
            set_value: Rc::new(set_value),
            default_value: None,
        }
    }

    /// Set the default value for this setting field, default is None.
    ///
    /// If set, this value can be used to reset the setting to its default state.
    /// If not set, the setting cannot be reset.
    pub fn default_value(mut self, default_value: T) -> Self {
        self.default_value = Some(default_value);
        self
    }
}

pub trait AnySettingField {
    fn as_any(&self) -> &dyn std::any::Any;
    fn type_name(&self) -> &'static str;
    fn type_id(&self) -> std::any::TypeId;
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
}

#[derive(Clone)]
pub enum SettingItem {
    /// A normal setting item with a title, description, and field.
    Item {
        title: SharedString,
        description: Option<SharedString>,
        field_type: SettingFieldType,
        field: Rc<dyn AnySettingField>,
    },
    /// A full custom element to render.
    Element {
        render: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>,
    },
}

impl SettingItem {
    pub(crate) fn is_match(&self, query: &str) -> bool {
        match self {
            SettingItem::Item {
                title, description, ..
            } => {
                title.to_lowercase().contains(&query.to_lowercase())
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
        title: SharedString,
        description: Option<SharedString>,
        field_type: SettingFieldType,
        field: Rc<dyn AnySettingField>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let type_id = field.deref().type_id();
        let renderer: Box<dyn SettingFieldRender> = match type_id {
            t if t == std::any::TypeId::of::<bool>() => {
                Box::new(BoolField::new(field_type.is_switch()))
            }
            t if t == TypeId::of::<f64>() && field_type.is_number_input() => {
                Box::new(NumberField::new(field_type.number_input_options()))
            }
            t if t == TypeId::of::<SharedString>() && field_type.is_input() => {
                Box::new(StringField::<SharedString>::new())
            }
            t if t == TypeId::of::<String>() && field_type.is_input() => {
                Box::new(StringField::<String>::new())
            }
            t if t == TypeId::of::<SharedString>() && field_type.is_dropdown() => Box::new(
                DropdownField::<SharedString>::new(field_type.dropdown_options()),
            ),
            t if t == TypeId::of::<String>() && field_type.is_dropdown() => {
                Box::new(DropdownField::<String>::new(field_type.dropdown_options()))
            }
            _ => unimplemented!(
                "Unsupported setting type: {} and field_type: {:?}",
                field.deref().type_name(),
                field_type
            ),
        };

        renderer.render(title, description, field, window, cx)
    }

    pub(super) fn render(self, ix: usize, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(SharedString::from(format!("item-{}", ix)))
            .child(match self {
                SettingItem::Item {
                    title,
                    description,
                    field_type,
                    field,
                } => {
                    h_flex()
                        .gap_4()
                        .justify_between()
                        .items_start()
                        .flex_wrap()
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_1()
                                .max_w_3_5()
                                .child(Label::new(title.clone()))
                                .when_some(description.clone(), |this, description| {
                                    this.child(
                                        Label::new(description)
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                }),
                        )
                        .child(div().id("field").bg(cx.theme().background).child(
                            Self::render_field(title, description, field_type, field, window, cx),
                        ))
                        .into_any_element()
                }
                SettingItem::Element { render } => (render)(window, cx).into_any_element(),
            })
    }
}
