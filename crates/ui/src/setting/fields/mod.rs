mod bool;
mod dropdown;
mod element;
mod number;
mod string;

pub(crate) use bool::*;
pub(crate) use dropdown::*;
pub(crate) use element::*;
pub(crate) use number::*;
pub(crate) use string::*;

pub use number::NumberFieldOptions;

use gpui::{AnyElement, App, Axis, IntoElement, SharedString, StyleRefinement, Styled, Window};
use std::{any::Any, rc::Rc};

use crate::Size;

pub(crate) trait SettingFieldRender {
    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        field: Rc<dyn AnySettingField>,
        size: Size,
        layout: Axis,
        style: &StyleRefinement,
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
) -> Rc<dyn Fn(T, &mut App)> {
    let setting_field = field
        .as_any()
        .downcast_ref::<SettingField<T>>()
        .expect("Failed to downcast setting field");
    setting_field.set_value.clone()
}

/// The type of setting field to render.
#[derive(Clone)]
pub enum SettingFieldType {
    Switch,
    Checkbox,
    NumberInput {
        options: NumberFieldOptions,
    },
    Input,
    Dropdown {
        options: Vec<(SharedString, SharedString)>,
    },
    Element {
        element_render: Rc<dyn Fn(Size, &mut Window, &mut App) -> AnyElement>,
    },
}

impl SettingFieldType {
    #[inline]
    pub(crate) fn is_switch(&self) -> bool {
        matches!(self, SettingFieldType::Switch)
    }

    #[inline]
    pub(crate) fn is_number_input(&self) -> bool {
        matches!(self, SettingFieldType::NumberInput { .. })
    }

    #[inline]
    pub(crate) fn is_input(&self) -> bool {
        matches!(self, SettingFieldType::Input)
    }

    #[inline]
    pub(crate) fn is_dropdown(&self) -> bool {
        matches!(self, SettingFieldType::Dropdown { .. })
    }

    #[inline]
    pub(crate) fn is_element(&self) -> bool {
        matches!(self, SettingFieldType::Element { .. })
    }

    #[inline]
    pub(super) fn dropdown_options(&self) -> Option<&Vec<(SharedString, SharedString)>> {
        match self {
            SettingFieldType::Dropdown { options } => Some(options),
            _ => None,
        }
    }

    #[inline]
    pub(super) fn number_input_options(&self) -> Option<&NumberFieldOptions> {
        match self {
            SettingFieldType::NumberInput { options } => Some(options),
            _ => None,
        }
    }

    #[inline]
    pub(super) fn element_render(
        &self,
    ) -> Rc<dyn Fn(Size, &mut Window, &mut App) -> AnyElement + 'static> {
        match self {
            SettingFieldType::Element { element_render } => element_render.clone(),
            _ => unreachable!("element_render called on non-element field"),
        }
    }
}

/// A setting field that can get and set a value of type T in the App.
pub struct SettingField<T> {
    pub(crate) field_type: SettingFieldType,
    pub(crate) style: StyleRefinement,
    /// Function to get the value for this field.
    pub(crate) value: Rc<dyn Fn(&App) -> T>,
    /// Function to set the value for this field.
    pub(crate) set_value: Rc<dyn Fn(T, &mut App)>,
    pub(crate) default_value: Option<T>,
}

impl SettingField<bool> {
    /// Create a new Switch field.
    pub fn switch<V, S>(value: V, set_value: S) -> Self
    where
        V: Fn(&App) -> bool + 'static,
        S: Fn(bool, &mut App) + 'static,
    {
        Self::new(SettingFieldType::Switch, value, set_value)
    }

    /// Create a new Checkbox field.
    pub fn checkbox<V, S>(value: V, set_value: S) -> Self
    where
        V: Fn(&App) -> bool + 'static,
        S: Fn(bool, &mut App) + 'static,
    {
        Self::new(SettingFieldType::Checkbox, value, set_value)
    }
}

impl SettingField<SharedString> {
    /// Create a new Input field.
    pub fn input<V, S>(value: V, set_value: S) -> Self
    where
        V: Fn(&App) -> SharedString + 'static,
        S: Fn(SharedString, &mut App) + 'static,
    {
        Self::new(SettingFieldType::Input, value, set_value)
    }

    /// Create a new Dropdown field with the given options.
    pub fn dropdown<V, S>(
        options: Vec<(SharedString, SharedString)>,
        value: V,
        set_value: S,
    ) -> Self
    where
        V: Fn(&App) -> SharedString + 'static,
        S: Fn(SharedString, &mut App) + 'static,
    {
        Self::new(SettingFieldType::Dropdown { options }, value, set_value)
    }

    /// Create a new setting field with the given element render function.
    pub fn element<R, E>(element_render: R) -> Self
    where
        E: IntoElement,
        R: Fn(Size, &mut Window, &mut App) -> E + 'static,
    {
        Self::new(
            SettingFieldType::Element {
                element_render: Rc::new(move |size, window, cx| {
                    element_render(size, window, cx).into_any_element()
                }),
            },
            |_| SharedString::default(),
            |_, _| {},
        )
    }
}

impl SettingField<f64> {
    /// Create a new Number Input field with the given options.
    pub fn number_input<V, S>(options: NumberFieldOptions, value: V, set_value: S) -> Self
    where
        V: Fn(&App) -> f64 + 'static,
        S: Fn(f64, &mut App) + 'static,
    {
        Self::new(SettingFieldType::NumberInput { options }, value, set_value)
    }
}

impl<T> SettingField<T> {
    /// Create a new setting field with the given get and set functions.
    fn new<V, S>(field_type: SettingFieldType, value: V, set_value: S) -> Self
    where
        V: Fn(&App) -> T + 'static,
        S: Fn(T, &mut App) + 'static,
    {
        Self {
            field_type,
            style: StyleRefinement::default(),
            value: Rc::new(value),
            set_value: Rc::new(set_value),
            default_value: None,
        }
    }

    /// Set the default value for this setting field, default is None.
    ///
    /// If set, this value can be used to reset the setting to its default state.
    /// If not set, the setting cannot be reset.
    pub fn default_value(mut self, default_value: impl Into<T>) -> Self {
        self.default_value = Some(default_value.into());
        self
    }
}

impl<T> Styled for SettingField<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A trait for setting fields that allows for dynamic typing.
pub trait AnySettingField {
    fn as_any(&self) -> &dyn std::any::Any;
    fn type_name(&self) -> &'static str;
    fn type_id(&self) -> std::any::TypeId;
    fn field_type(&self) -> &SettingFieldType;
    fn style(&self) -> &StyleRefinement;
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

    fn field_type(&self) -> &SettingFieldType {
        &self.field_type
    }

    fn style(&self) -> &StyleRefinement {
        &self.style
    }
}
