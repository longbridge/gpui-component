use gpui::Window;

use super::InputContentType;

pub(crate) fn set_text_content_type(window: &Window, content_type: Option<InputContentType>) {
    gpui_base::input::set_text_content_type(
        window,
        content_type.and_then(InputContentType::ns_text_content_type),
    );
}
