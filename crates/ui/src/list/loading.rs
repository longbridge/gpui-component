use super::ListItem;
use crate::{skeleton::Skeleton, v_flex};
use gpui::{IntoElement, ParentElement as _, RenderOnce, Styled};

#[derive(IntoElement)]
pub struct Loading;

#[derive(IntoElement)]
struct LoadingItem;

impl RenderOnce for LoadingItem {
    fn render(self, _: &mut gpui::WindowContext) -> impl IntoElement {
        ListItem::new("skeleton").disabled(true).child(
            v_flex()
                .gap_1p5()
                .overflow_hidden()
                .child(Skeleton::new().h_5().w_48().max_w_full())
                .child(Skeleton::new().secondary(true).h_3().w_64().max_w_full()),
        )
    }
}

impl RenderOnce for Loading {
    fn render(self, _: &mut gpui::WindowContext) -> impl IntoElement {
        v_flex()
            .py_2p5()
            .gap_3()
            .child(LoadingItem)
            .child(LoadingItem)
            .child(LoadingItem)
    }
}
