use super::schema::{PaginationEvent, PaginationProps};
use gpui_kit::{
    AnyElement, App, IntoElement, Window,
    component::{Disableable, Sizable, pagination::Pagination},
    prelude::FluentBuilder,
};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
pub(super) struct NativePagination {
    context: TypedComponentContext<PaginationEvent>,
}
impl NativePagination {
    pub(super) fn new(context: TypedComponentContext<PaginationEvent>) -> Self {
        Self { context }
    }
}
impl TypedNativeComponent for NativePagination {
    type Props = PaginationProps;
    type Event = PaginationEvent;
    fn validate_props(&self, p: &PaginationProps) -> Result<(), nsh_core::ui::PropError> {
        if p.page == 0 || p.total_pages == 0 || p.visible_pages == 0 {
            return Err(nsh_core::ui::PropError(
                "Pagination pages must be positive integers".into(),
            ));
        }
        Ok(())
    }
    fn render(&mut self, p: &PaginationProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let context = self.context.clone();
        Some(
            Pagination::new("pagination")
                .current_page(p.page)
                .total_pages(p.total_pages)
                .visible_pages(p.visible_pages)
                .disabled(p.disabled)
                .with_size(p.size.resolve())
                .when(p.compact, |element| element.compact())
                .on_click(move |page, _, _| context.emit(PaginationEvent::Change(*page)))
                .into_any_element(),
        )
    }
}
