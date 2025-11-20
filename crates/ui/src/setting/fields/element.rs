use gpui::{AnyElement, App, StyleRefinement, Window};
use std::rc::Rc;

use crate::setting::{fields::SettingFieldRender, AnySettingField, RenderOptions};

pub(crate) struct ElementField {
    element_render: Rc<dyn Fn(&RenderOptions, &mut Window, &mut App) -> AnyElement>,
}

impl ElementField {
    pub(crate) fn new(
        element_render: Rc<dyn Fn(&RenderOptions, &mut Window, &mut App) -> AnyElement + 'static>,
    ) -> Self {
        Self { element_render }
    }
}

impl SettingFieldRender for ElementField {
    fn render(
        &self,
        _: Rc<dyn AnySettingField>,
        options: &RenderOptions,
        _style: &StyleRefinement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        (self.element_render)(options, window, cx)
    }
}
