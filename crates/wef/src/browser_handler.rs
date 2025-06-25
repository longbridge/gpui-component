use std::{
    ffi::{CStr, c_char, c_void},
    mem::MaybeUninit,
};

use num_enum::TryFromPrimitive;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    Accept, ContextMenuEditStateFlags, ContextMenuMediaStateFlags, ContextMenuMediaType,
    ContextMenuParams, ContextMenuTypeFlags, CursorType, DirtyRects, FileDialogCallback,
    FileDialogMode, Frame, JsDialogCallback, JsDialogType, LogicalUnit, Point, Rect, Size,
    builder::BrowserState, cursor::CursorInfo, ffi::*, file_dialog::AcceptFilter,
    query::QueryCallback,
};

/// A type alias for the image buffer.
pub type ImageBuffer<'a> = image::ImageBuffer<image::Rgba<u8>, &'a [u8]>;

/// Paint element types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[allow(missing_docs)]
#[repr(i32)]
pub enum PaintElementType {
    View = 0,
    Popup = 1,
}

/// Log severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum LogSeverity {
    /// Default logging (currently INFO logging).
    Default = 0,
    /// DEBUG logging.
    Debug = 1,
    /// INFO logging.
    Info = 2,
    /// WARNING logging.
    Warning = 3,
    /// ERROR logging.
    Error = 4,
    /// FATAL logging.
    Fatal = 5,
    /// Disable logging to file for all messages, and to stderr for messages
    /// with severity less than FATAL.
    Disable = 99,
}

/// Represents a handler for browser events.
///
/// **All the functions are called on the main thread.**
#[allow(unused_variables)]
pub trait BrowserHandler {
    /// Called when the browser is created.
    fn on_created(&mut self) {}

    /// Called when the browser is closed.
    fn on_closed(&mut self) {}

    /// Called when the browser wants to show or hide the popup widget.
    fn on_popup_show(&mut self, show: bool) {}

    /// Called when the browser wants to move or resize the popup widget.
    fn on_popup_position(&mut self, rect: Rect<LogicalUnit<i32>>) {}

    /// Called when an element should be painted.
    ///
    /// Pixel values passed to this method are scaled relative to view
    /// coordinates based on the value of
    /// [`crate::BrowserBuilder::device_scale_factor`].
    ///
    /// `type` indicates whether the element is the view or the popup widget.
    ///
    /// `image_buffer` contains the pixel data for the whole image.
    ///
    /// `dirty_rects` contains the set of rectangles in pixel coordinates that
    /// need to be repainted.
    fn on_paint(
        &mut self,
        type_: PaintElementType,
        dirty_rects: &DirtyRects,
        image_buffer: ImageBuffer,
    ) {
    }

    /// Called when the address of the frame changes.
    fn on_address_changed(&mut self, frame: Frame, url: &str) {}

    /// Called when the title changes.
    fn on_title_changed(&mut self, title: &str) {}

    /// Called when the page icon changes.
    fn on_favicon_url_changed(&mut self, urls: &[&str]) {}

    /// Called when the browser is about to display a tooltip.
    fn on_tooltip(&mut self, text: &str) {}

    /// Called when the browser receives a status message.
    fn on_status_message(&mut self, text: &str) {}

    /// Called to display a console message.
    fn on_console_message(
        &mut self,
        message: &str,
        level: LogSeverity,
        source: &str,
        line_number: i32,
    ) {
    }

    /// Called when the cursor changes.
    ///
    /// Return `true` if the cursor change was handled or false for default
    /// handling.
    fn on_cursor_changed(
        &mut self,
        cursor_type: CursorType,
        cursor_info: Option<CursorInfo>,
    ) -> bool {
        false
    }

    /// Called when preparing to open a popup browser window.
    fn on_before_popup(&mut self, url: &str) {}

    /// Called when the overall page loading progress changes.
    ///
    /// `progress` ranges from 0.0 to 1.0.
    fn on_loading_progress_changed(&mut self, progress: f32) {}

    /// Called when the loading state changes.
    ///
    /// This callback will be executed twice -- once when loading is initiated
    /// either programmatically or by user action, and once when loading is
    /// terminated due to completion, cancellation of failure.
    ///
    /// It will be called before any calls to `on_load_start` and after all
    /// calls to `on_load_error` and/or `on_load_end`.
    fn on_loading_state_changed(
        &mut self,
        is_loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    ) {
    }

    /// Called after a navigation has been committed and before the browser
    /// begins loading contents in the frame.
    fn on_load_start(&mut self, frame: Frame) {}

    /// Called when the browser is done loading a frame.
    fn on_load_end(&mut self, frame: Frame) {}

    /// Called when a navigation fails or is canceled.
    ///
    /// This method may be called by itself if before commit or in combination
    /// with `on_load_start`/`on_load_end` if after commit.
    fn on_load_error(&mut self, frame: Frame, error_text: &str, failed_url: &str) {}

    /// Called when the IME composition range changes.
    fn on_ime_composition_range_changed(&mut self, bounds: Rect<LogicalUnit<i32>>) {}

    /// Called when a file dialog is requested.
    ///
    /// To display the default dialog return `false`.
    fn on_file_dialog(
        &mut self,
        mode: FileDialogMode,
        title: Option<&str>,
        default_file_path: Option<&str>,
        accepts: &[Accept],
        callback: FileDialogCallback,
    ) -> bool {
        false
    }

    /// Called when before displaying a context menu.
    fn on_context_menu(&mut self, frame: Frame, params: ContextMenuParams) {}

    /// Called to report find results returned by [`crate::Browser::find`].
    ///
    /// `identifier`` is a unique incremental identifier for the currently
    /// active search.
    /// `count`` is the number of matches currently identified.
    /// `selection_rect` is the location of where the match was found (in window
    /// coordinates).
    /// `active_match_ordinal` is the current position in the search results.
    /// `final_update` is `true` if this is the last find notification.
    fn on_find_result(
        &mut self,
        identifier: i32,
        count: i32,
        selection_rect: Rect<LogicalUnit<i32>>,
        active_match_ordinal: i32,
        final_update: bool,
    ) {
    }

    /// Called to run a JavaScript dialog.
    ///
    /// The `default_prompt_text` value will be specified for prompt dialogs
    /// only.
    ///
    /// Return `true` if the application will use a custom
    /// dialog or if the callback has been executed immediately. Custom dialogs
    /// may be either modal or modeless.
    ///
    /// If a custom dialog is used the application must execute `callback` once
    /// the custom dialog is dismissed.
    fn on_js_dialog(
        &mut self,
        type_: JsDialogType,
        message_text: &str,
        callback: JsDialogCallback,
    ) -> bool {
        false
    }
}

impl BrowserHandler for () {}

pub(crate) extern "C" fn on_created<T: BrowserHandler>(userdata: *mut c_void) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state.handler.on_created();
    }
}

pub(crate) extern "C" fn on_closed<T: BrowserHandler>(userdata: *mut c_void) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state.handler.on_closed();
    }
}

pub(crate) extern "C" fn on_popup_show<T: BrowserHandler>(userdata: *mut c_void, show: bool) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state.handler.on_popup_show(show);
    }
}

pub(crate) extern "C" fn on_popup_position<T: BrowserHandler>(
    userdata: *mut c_void,
    rect: *const Rect<i32>,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state.handler.on_popup_position((*rect).map(LogicalUnit));
    }
}

pub(crate) extern "C" fn on_paint<T: BrowserHandler>(
    userdata: *mut c_void,
    type_: i32,
    dirty_rects: *const c_void,
    image_buffer: *const c_void,
    width: u32,
    height: u32,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let type_ = PaintElementType::try_from(type_).expect("BUG: invalid paint element type");
        let dirty_rects = DirtyRects::new(dirty_rects);
        let image_buffer =
            std::slice::from_raw_parts(image_buffer as *const u8, (width * height * 4) as usize);
        state.handler.on_paint(
            type_,
            &dirty_rects,
            ImageBuffer::from_raw(width, height, image_buffer).unwrap(),
        );
    }
}

pub(crate) extern "C" fn on_address_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    frame: *mut wef_frame_t,
    url: *const c_char,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let frame = Frame(frame);
        let url = CStr::from_ptr(url).to_string_lossy();
        state.handler.on_address_changed(frame, &url);
    }
}

pub(crate) extern "C" fn on_title_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    title: *const c_char,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let title = CStr::from_ptr(title).to_string_lossy();
        state.handler.on_title_changed(&title);
    }
}

pub(crate) extern "C" fn on_favicon_url_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    urls: *const *const c_char,
    num_urls: i32,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let urls = std::slice::from_raw_parts(urls, num_urls as usize)
            .iter()
            .filter_map(|url| CStr::from_ptr(*url).to_str().ok())
            .collect::<Vec<_>>();
        state.handler.on_favicon_url_changed(&urls);
    }
}

pub(crate) extern "C" fn on_tooltip<T: BrowserHandler>(userdata: *mut c_void, text: *const c_char) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let text = CStr::from_ptr(text).to_string_lossy();
        state.handler.on_tooltip(&text);
    }
}

pub(crate) extern "C" fn on_status_message<T: BrowserHandler>(
    userdata: *mut c_void,
    text: *const c_char,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let text = CStr::from_ptr(text).to_string_lossy();
        state.handler.on_status_message(&text);
    }
}

pub(crate) extern "C" fn on_console_message<T: BrowserHandler>(
    userdata: *mut c_void,
    message: *const c_char,
    level: i32,
    source: *const c_char,
    line: i32,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let message = CStr::from_ptr(message).to_string_lossy();
        let source = CStr::from_ptr(source).to_string_lossy();
        let level = LogSeverity::try_from(level).expect("BUG: invalid log severity");
        state
            .handler
            .on_console_message(&message, level, &source, line);
    }
}

pub(crate) extern "C" fn on_cursor_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    cursor_type: i32,
    custom_cursor_info: *const c_void,
) -> bool {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let cursor_type = CursorType::try_from(cursor_type).expect("BUG: invalid file dialog mode");
        let cursor_info = if !custom_cursor_info.is_null() {
            let mut hotspot: MaybeUninit<Point<i32>> = MaybeUninit::uninit();
            let mut size: MaybeUninit<Size<i32>> = MaybeUninit::uninit();

            wef_cursor_info_hotspot(custom_cursor_info, hotspot.as_mut_ptr());
            wef_cursor_info_size(custom_cursor_info, size.as_mut_ptr());

            let hotspot = hotspot.assume_init();
            let size = size.assume_init();

            let image_buffer = std::slice::from_raw_parts(
                wef_cursor_info_buffer(custom_cursor_info) as *const u8,
                (size.width * size.height * 4) as usize,
            );

            Some(CursorInfo {
                hotspot,
                scale_factor: wef_cursor_info_image_scale_factor(custom_cursor_info),
                image: ImageBuffer::from_raw(size.width as u32, size.height as u32, image_buffer)
                    .expect("BUG: invalid image buffer size"),
            })
        } else {
            None
        };
        state.handler.on_cursor_changed(cursor_type, cursor_info)
    }
}

pub(crate) extern "C" fn on_before_popup<T: BrowserHandler>(
    userdata: *mut c_void,
    url: *const c_char,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let url = CStr::from_ptr(url).to_string_lossy();
        state.handler.on_before_popup(&url);
    }
}

pub(crate) extern "C" fn on_loading_progress_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    progress: f32,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state.handler.on_loading_progress_changed(progress);
    }
}

pub(crate) extern "C" fn on_loading_state_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    is_loading: bool,
    can_go_back: bool,
    can_go_forward: bool,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state
            .handler
            .on_loading_state_changed(is_loading, can_go_back, can_go_forward);
    }
}

pub(crate) extern "C" fn on_load_start<T: BrowserHandler>(
    userdata: *mut c_void,
    frame: *mut wef_frame_t,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let frame = Frame(frame);
        state.handler.on_load_start(frame);
    }
}

pub(crate) extern "C" fn on_load_end<T: BrowserHandler>(
    userdata: *mut c_void,
    frame: *mut wef_frame_t,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let frame = Frame(frame);
        state.handler.on_load_end(frame);
    }
}

pub(crate) extern "C" fn on_load_error<T: BrowserHandler>(
    userdata: *mut c_void,
    frame: *mut wef_frame_t,
    error_text: *const c_char,
    failed_url: *const c_char,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let frame = Frame(frame);
        let error_text = CStr::from_ptr(error_text).to_string_lossy();
        let failed_url = CStr::from_ptr(failed_url).to_string_lossy();
        state.handler.on_load_error(frame, &error_text, &failed_url);
    }
}

pub(crate) extern "C" fn on_ime_composition_range_changed<T: BrowserHandler>(
    userdata: *mut c_void,
    bounds: *const Rect<i32>,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state
            .handler
            .on_ime_composition_range_changed((*bounds).map(LogicalUnit));
    }
}

pub(crate) extern "C" fn on_file_dialog<T: BrowserHandler>(
    userdata: *mut c_void,
    mode: i32,
    title: *const c_char,
    default_file_path: *const c_char,
    accept_filters: *const c_char,
    accept_extensions: *const c_char,
    accept_descriptions: *const c_char,
    callback: *mut c_void,
) -> bool {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let mode = FileDialogMode::try_from(mode).expect("BUG: invalid file dialog mode");
        let title = CStr::from_ptr(title).to_string_lossy();
        let default_file_path = CStr::from_ptr(default_file_path).to_string_lossy();
        let accept_filters = CStr::from_ptr(accept_filters).to_string_lossy();
        let accept_extensions = CStr::from_ptr(accept_extensions).to_string_lossy();
        let accept_descriptions = CStr::from_ptr(accept_descriptions).to_string_lossy();
        let mut extensions_vec = vec![];
        let mut accepts = vec![];
        const DELIMITER: &str = "@@@";

        for ((filter, extensions), description) in accept_filters
            .split(DELIMITER)
            .zip(accept_extensions.split(DELIMITER))
            .zip(accept_descriptions.split(DELIMITER))
        {
            let filter = if filter.starts_with('.') {
                AcceptFilter::Extension(filter)
            } else {
                let Ok(mime) = filter.parse() else {
                    continue;
                };
                AcceptFilter::Mime(mime)
            };

            let extensions = (!extensions.is_empty()).then(|| {
                extensions_vec.push(extensions.split(';').collect::<Vec<_>>());
                extensions_vec.len() - 1
            });

            let description = (!description.is_empty()).then_some(description);
            accepts.push((filter, extensions, description));
        }

        let accepts = accepts
            .into_iter()
            .map(|(filter, extensions, description)| Accept {
                filters: filter,
                extensions: extensions.map(|idx| &*extensions_vec[idx]),
                description,
            })
            .collect::<Vec<_>>();

        let title = (!title.is_empty()).then_some(&*title);
        let default_file_path = (!default_file_path.is_empty()).then_some(&*default_file_path);

        state.handler.on_file_dialog(
            mode,
            title,
            default_file_path,
            &accepts,
            FileDialogCallback::new(callback),
        )
    }
}

pub(crate) extern "C" fn on_context_menu<T: BrowserHandler>(
    userdata: *mut c_void,
    frame: *mut wef_frame_t,
    params: *const CContextMenuParams,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let frame = Frame(frame);
        let link_url = (!(*params).link_url.is_null())
            .then(|| CStr::from_ptr((*params).link_url).to_string_lossy());
        let unfiltered_link_url = (!(*params).unfiltered_link_url.is_null())
            .then(|| CStr::from_ptr((*params).unfiltered_link_url).to_string_lossy());
        let source_url = (!(*params).source_url.is_null())
            .then(|| CStr::from_ptr((*params).source_url).to_string_lossy());
        let title_text = (!(*params).title_text.is_null())
            .then(|| CStr::from_ptr((*params).title_text).to_string_lossy());
        let page_url = CStr::from_ptr((*params).page_url).to_string_lossy();
        let frame_url = CStr::from_ptr((*params).frame_url).to_string_lossy();
        let selection_text = (!(*params).selection_text.is_null())
            .then(|| CStr::from_ptr((*params).selection_text).to_string_lossy());

        state.handler.on_context_menu(
            frame,
            ContextMenuParams {
                crood: Point::new(
                    LogicalUnit((*params).x_crood),
                    LogicalUnit((*params).y_crood),
                ),
                type_: ContextMenuTypeFlags::from_bits_truncate((*params).type_flags),
                link_url: link_url.as_deref(),
                unfiltered_link_url: unfiltered_link_url.as_deref(),
                source_url: source_url.as_deref(),
                has_image_contents: (*params).has_image_contents,
                title_text: title_text.as_deref(),
                page_url: &page_url,
                frame_url: &frame_url,
                media_type: ContextMenuMediaType::try_from((*params).media_type)
                    .unwrap_or_default(),
                media_state_flags: ContextMenuMediaStateFlags::from_bits_truncate(
                    (*params).media_state_flags,
                ),
                selection_text: selection_text.as_deref(),
                is_editable: (*params).is_editable,
                edit_state_flags: ContextMenuEditStateFlags::from_bits_truncate(
                    (*params).edit_state_flags,
                ),
            },
        );
    }
}

pub(crate) extern "C" fn on_find_result<T: BrowserHandler>(
    userdata: *mut c_void,
    identifier: i32,
    count: i32,
    selection_rect: *const Rect<i32>,
    active_match_ordinal: i32,
    final_update: bool,
) {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        state.handler.on_find_result(
            identifier,
            count,
            (*selection_rect).map(LogicalUnit),
            active_match_ordinal,
            final_update,
        );
    }
}

pub(crate) extern "C" fn on_js_dialog<T: BrowserHandler>(
    userdata: *mut c_void,
    type_: i32,
    message_text: *const c_char,
    default_prompt_text: *const c_char,
    callback: *mut c_void,
) -> bool {
    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let default_prompt_text = CStr::from_ptr(default_prompt_text).to_string_lossy();
        let type_ = match type_ {
            0 => JsDialogType::Alert,
            1 => JsDialogType::Confirm,
            2 => JsDialogType::Prompt {
                default_prompt_text: &default_prompt_text,
            },
            _ => panic!("BUG: invalid js dialog type"),
        };
        let message_text = CStr::from_ptr(message_text).to_string_lossy();
        state
            .handler
            .on_js_dialog(type_, &message_text, JsDialogCallback::new(callback))
    }
}

pub(crate) extern "C" fn on_query<T: BrowserHandler>(
    userdata: *mut c_void,
    frame: *mut wef_frame_t,
    query: *const c_char,
    callback: *mut wef_query_callback_t,
) {
    #[derive(Debug, Deserialize)]
    struct Request {
        method: String,
        args: Vec<Value>,
    }

    let frame = Frame(frame);

    unsafe {
        let state = &mut *(userdata as *mut BrowserState<T>);
        let Some(request) = CStr::from_ptr(query)
            .to_str()
            .ok()
            .and_then(|value| serde_json::from_str::<Request>(value).ok())
        else {
            return;
        };

        state.func_registry.call(
            frame,
            &request.method,
            request.args,
            QueryCallback::new(callback),
        )
    }
}
