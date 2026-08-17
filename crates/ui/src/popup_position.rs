use gpui::{Bounds, Pixels, px};

pub(crate) fn dropdown_positioner(bounds: Bounds<Pixels>) -> gpui_base::Positioner {
    gpui_base::Positioner::side(bounds)
        .placement(gpui_base::Placement::Bottom)
        .align(gpui_base::Align::Start)
        .offset(px(6.))
        .margin(px(8.))
}
