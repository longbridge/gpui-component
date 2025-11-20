use std::rc::Rc;

use gpui::{AnyElement, App, Axis, StyleRefinement, Window};

use crate::{
    setting::{fields::SettingFieldRender, AnySettingField},
    Size,
};

pub(crate) struct ElementField {
    element_render: Rc<dyn Fn(Size, &mut Window, &mut App) -> AnyElement>,
}

impl ElementField {
    pub(crate) fn new(
        element_render: Rc<dyn Fn(Size, &mut Window, &mut App) -> AnyElement + 'static>,
    ) -> Self {
        Self { element_render }
    }
}

impl SettingFieldRender for ElementField {
    fn render(
        &self,
        _: Rc<dyn AnySettingField>,
        size: Size,
        _: Axis,
        _: &StyleRefinement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        (self.element_render)(size, window, cx)
    }
}
