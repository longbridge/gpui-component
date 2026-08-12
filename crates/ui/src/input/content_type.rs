use gpui::Window;

pub use gpui_base::InputContentType;

pub(super) fn sync_native_content_type(
    window: &mut Window,
    content_type: Option<InputContentType>,
    disabled: bool,
) {
    if disabled {
        return;
    }

    #[cfg(all(target_os = "macos", not(test)))]
    gpui_base::input::set_text_content_type(window, content_type);

    #[cfg(any(not(target_os = "macos"), test))]
    let _ = (window, content_type);
}
