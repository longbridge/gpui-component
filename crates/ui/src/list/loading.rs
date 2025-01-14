use super::ListItem;
use crate::{skeleton::Skeleton, v_flex};
use gpui::{IntoElement, ParentElement as _, RenderOnce, Styled};

#[derive(IntoElement)]
pub struct Loading;

#[derive(IntoElement)]
struct LoadingRowItem;

impl RenderOnce for LoadingRowItem {
    fn render(self, _: &mut gpui::WindowContext) -> impl IntoElement {
        ListItem::new("skeleton").disabled(true).py_3().child(
            v_flex()
                .gap_1p5()
                .child(Skeleton::new().h_5().w_48().max_w_full())
                .child(Skeleton::new().secondary().h_3().w_64().max_w_full()),
        )
    }
}

impl RenderOnce for Loading {
    fn render(self, _: &mut gpui::WindowContext) -> impl IntoElement {
        v_flex()
            .child(LoadingRowItem)
            .child(LoadingRowItem)
            .child(LoadingRowItem)
    }
}
