use gpui::{
    div, prelude::FluentBuilder as _, AnyElement, App, Axis, InteractiveElement as _, IntoElement,
    ParentElement, SharedString, Styled, Window,
};
use std::{any::TypeId, ops::Deref, rc::Rc};

use crate::{
    label::Label,
    setting::{
        fields::{BoolField, DropdownField, NumberField, SettingFieldRender, StringField},
        AnySettingField, ElementField, RenderOptions,
    },
    text::Text,
    v_flex, ActiveTheme as _, AxisExt, Size, StyledExt as _,
};

/// Setting item.
#[derive(Clone)]
pub enum SettingItem {
    /// A normal setting item with a title, description, and field.
    Item {
        title: SharedString,
        description: Option<Text>,
        layout: Axis,
        field: Rc<dyn AnySettingField>,
    },
    /// A full custom element to render.
    Element {
        render: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>,
    },
}

impl SettingItem {
    /// Create a new setting item.
    pub fn new<F>(title: impl Into<SharedString>, field: F) -> Self
    where
        F: AnySettingField + 'static,
    {
        SettingItem::Item {
            title: title.into(),
            description: None,
            layout: Axis::Horizontal,
            field: Rc::new(field),
        }
    }

    /// Create a new custom element setting item.
    pub fn element<R, E>(render: R) -> Self
    where
        E: IntoElement,
        R: Fn(&mut Window, &mut App) -> E + 'static,
    {
        SettingItem::Element {
            render: Rc::new(move |window, cx| render(window, cx).into_any_element()),
        }
    }

    /// Set the description of the setting item.
    ///
    /// Only applies to [`SettingItem::Item`].
    pub fn description(mut self, description: impl Into<Text>) -> Self {
        match &mut self {
            SettingItem::Item { description: d, .. } => {
                *d = Some(description.into());
            }
            SettingItem::Element { .. } => {}
        }
        self
    }

    /// Set the layout of the setting item.
    ///
    /// Only applies to [`SettingItem::Item`].
    pub fn layout(mut self, layout: Axis) -> Self {
        match &mut self {
            SettingItem::Item { layout: l, .. } => {
                *l = layout;
            }
            SettingItem::Element { .. } => {}
        }
        self
    }

    pub(crate) fn is_match(&self, query: &str) -> bool {
        match self {
            SettingItem::Item {
                title, description, ..
            } => {
                title.to_lowercase().contains(&query.to_lowercase())
                    || description.as_ref().map_or(false, |d| {
                        d.as_str().to_lowercase().contains(&query.to_lowercase())
                    })
            }
            SettingItem::Element { .. } => {
                // We need to show all custom elements when not searching.
                if query.is_empty() {
                    true
                } else {
                    false
                }
            }
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
        field: Rc<dyn AnySettingField>,
        size: Size,
        layout: Axis,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let field_type = field.field_type();
        let style = field.style().clone();
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
            _ if field_type.is_element() => {
                Box::new(ElementField::new(field_type.element_render()))
            }
            _ => unimplemented!("Unsupported setting type: {}", field.deref().type_name()),
        };

        renderer.render(field, size, layout, &style, window, cx)
    }

    pub(super) fn render(
        self,
        ix: usize,
        options: &RenderOptions,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        div()
            .id(SharedString::from(format!("item-{}", ix)))
            .child(match self {
                SettingItem::Item {
                    title,
                    description,
                    layout,
                    field,
                } => div()
                    .map(|this| {
                        if layout.is_horizontal() {
                            this.h_flex().justify_between().items_start().flex_wrap()
                        } else {
                            this.v_flex().items_start()
                        }
                    })
                    .gap_3()
                    .w_full()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .max_w_3_5()
                            .child(Label::new(title.clone()))
                            .when_some(description.clone(), |this, description| {
                                this.child(
                                    div()
                                        .size_full()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(description),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("field")
                            .bg(cx.theme().background)
                            .child(Self::render_field(field, options.size, layout, window, cx)),
                    )
                    .into_any_element(),
                SettingItem::Element { render } => (render)(window, cx).into_any_element(),
            })
    }
}
