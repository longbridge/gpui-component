mod bool;
mod number;
mod string;
mod unknow;

pub(crate) use bool::*;
pub(crate) use number::*;
pub(crate) use string::*;
pub(crate) use unknow::*;

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window};

use crate::setting::AnySettingField;

pub(crate) trait SettingFieldRender {
    #[allow(unused)]
    fn render(
        &self,
        id: &'static str,
        label: SharedString,
        description: Option<SharedString>,
        field: Rc<dyn AnySettingField>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;
}
