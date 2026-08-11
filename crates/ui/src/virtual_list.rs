pub use gpui_component_base::{
    VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list,
};
pub(crate) use gpui_component_base::virtual_list;

use gpui::{Pixels, Point, Size};

impl crate::scroll::ScrollbarHandle for VirtualListScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        self.base_handle().offset()
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.base_handle().set_offset(offset);
    }

    fn content_size(&self) -> Size<Pixels> {
        self.base_handle().content_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_handle_is_the_base_type_and_a_scrollbar_handle() {
        fn accepts_base(_: gpui_component_base::VirtualListScrollHandle) {}
        fn accepts_scrollbar(_: impl crate::scroll::ScrollbarHandle) {}
        fn legacy_list_is_base(value: crate::VirtualList) {
            fn accepts_base(_: gpui_component_base::VirtualList) {}
            accepts_base(value);
        }

        let handle: crate::VirtualListScrollHandle = VirtualListScrollHandle::new();
        accepts_base(handle.clone());
        accepts_scrollbar(handle);
        let _ = legacy_list_is_base;
    }
}
