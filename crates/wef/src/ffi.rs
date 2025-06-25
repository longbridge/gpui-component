#![allow(non_camel_case_types)]

use std::ffi::{CStr, c_char, c_void};

use crate::{Point, Rect, Size};

pub(crate) type wef_browser_t = c_void;
pub(crate) type wef_frame_t = c_void;
pub(crate) type wef_cursor_info_t = c_void;
pub(crate) type wef_file_dialog_callback_t = c_void;
pub(crate) type wef_js_dialog_callback_t = c_void;
pub(crate) type wef_query_callback_t = c_void;

type DestroyFn = extern "C" fn(*mut c_void);

#[repr(C)]
pub(crate) struct CAppCallbacks {
    pub(crate) on_schedule_message_pump_work: extern "C" fn(*mut c_void, i32),
}

#[repr(C)]
pub(crate) struct CSettings {
    pub(crate) locale: *const c_char,
    pub(crate) cache_path: *const c_char,
    pub(crate) root_cache_path: *const c_char,
    pub(crate) browser_subprocess_path: *const c_char,
    pub(crate) callbacks: CAppCallbacks,
    pub(crate) userdata: *mut c_void,
    pub(crate) destroy_userdata: DestroyFn,
}

#[repr(C)]
pub(crate) struct CBrowserSettings {
    pub(crate) parent: *const c_void,
    pub(crate) device_scale_factor: f32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) frame_rate: i32,
    pub(crate) url: *const c_char,
    pub(crate) inject_javascript: *const c_char,
    pub(crate) callbacks: CBrowserCallbacks,
    pub(crate) userdata: *mut c_void,
    pub(crate) destroy_userdata: DestroyFn,
}

#[repr(C)]
pub(crate) struct CContextMenuParams {
    pub(crate) x_crood: i32,
    pub(crate) y_crood: i32,
    pub(crate) type_flags: i32,
    pub(crate) link_url: *const c_char,
    pub(crate) unfiltered_link_url: *const c_char,
    pub(crate) source_url: *const c_char,
    pub(crate) has_image_contents: bool,
    pub(crate) title_text: *const c_char,
    pub(crate) page_url: *const c_char,
    pub(crate) frame_url: *const c_char,
    pub(crate) media_type: i32,
    pub(crate) media_state_flags: i32,
    pub(crate) selection_text: *const c_char,
    pub(crate) is_editable: bool,
    pub(crate) edit_state_flags: i32,
}

#[repr(C)]
pub(crate) struct CBrowserCallbacks {
    pub(crate) on_created: extern "C" fn(*mut c_void),
    pub(crate) on_closed: extern "C" fn(*mut c_void),
    pub(crate) on_popup_show: extern "C" fn(*mut c_void, bool),
    pub(crate) on_popup_position: extern "C" fn(*mut c_void, *const Rect<i32>),
    pub(crate) on_paint: extern "C" fn(*mut c_void, i32, *const c_void, *const c_void, u32, u32),
    pub(crate) on_address_changed: extern "C" fn(*mut c_void, *mut wef_frame_t, *const c_char),
    pub(crate) on_title_changed: extern "C" fn(*mut c_void, *const c_char),
    pub(crate) on_favicon_url_changed: extern "C" fn(*mut c_void, *const *const c_char, i32),
    pub(crate) on_tooltip: extern "C" fn(*mut c_void, *const c_char),
    pub(crate) on_status_message: extern "C" fn(*mut c_void, *const c_char),
    pub(crate) on_console_message:
        extern "C" fn(*mut c_void, *const c_char, i32, *const c_char, i32),
    pub(crate) on_cursor_changed: extern "C" fn(
        *mut c_void,
        cursor_type: i32,
        custom_cursor_info: *const wef_cursor_info_t,
    ) -> bool,
    pub(crate) on_before_popup: extern "C" fn(*mut c_void, *const c_char),
    pub(crate) on_loading_progress_changed: extern "C" fn(*mut c_void, f32),
    pub(crate) on_loading_state_changed: extern "C" fn(*mut c_void, bool, bool, bool),
    pub(crate) on_load_start: extern "C" fn(*mut c_void, *mut wef_frame_t),
    pub(crate) on_load_end: extern "C" fn(*mut c_void, *mut wef_frame_t),
    pub(crate) on_load_error:
        extern "C" fn(*mut c_void, *mut wef_frame_t, *const c_char, *const c_char),
    pub(crate) on_ime_composition_range_changed: extern "C" fn(*mut c_void, *const Rect<i32>),
    pub(crate) on_file_dialog: extern "C" fn(
        *mut c_void,
        i32,
        *const c_char,
        *const c_char,
        *const c_char,
        *const c_char,
        *const c_char,
        *mut wef_file_dialog_callback_t,
    ) -> bool,
    pub(crate) on_context_menu:
        extern "C" fn(*mut c_void, *mut wef_frame_t, *const CContextMenuParams),
    pub(crate) on_find_result: extern "C" fn(*mut c_void, i32, i32, *const Rect<i32>, i32, bool),
    pub(crate) on_js_dialog: extern "C" fn(
        *mut c_void,
        i32,
        *const c_char,
        *const c_char,
        *mut wef_js_dialog_callback_t,
    ) -> bool,
    pub(crate) on_query:
        extern "C" fn(*mut c_void, *mut wef_frame_t, *const c_char, *mut wef_query_callback_t),
}

#[inline]
pub(crate) fn to_cstr_ptr_opt(s: Option<&CStr>) -> *const c_char {
    s.map(|s| s.as_ptr()).unwrap_or(std::ptr::null())
}

unsafe extern "C" {
    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn wef_load_library(helper: bool) -> *mut c_void;

    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn wef_unload_library(loader: *mut c_void);

    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn wef_sandbox_context_create(
        args: *const *const c_char,
        count: i32,
    ) -> *mut c_void;

    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn wef_sandbox_context_destroy(loader: *mut c_void);

    pub(crate) unsafe fn wef_init(settings: *const CSettings) -> bool;

    pub(crate) unsafe fn wef_exec_process(args: *const *const c_char, count: i32) -> bool;

    pub(crate) unsafe fn wef_shutdown();

    pub(crate) unsafe fn wef_do_message_work();

    pub(crate) unsafe fn wef_browser_create(
        settings: *const CBrowserSettings,
    ) -> *mut wef_browser_t;

    pub(crate) unsafe fn wef_browser_close(browser: *mut wef_browser_t);

    pub(crate) unsafe fn wef_browser_destroy(browser: *mut wef_browser_t);

    pub(crate) unsafe fn wef_browser_is_created(browser: *mut wef_browser_t) -> bool;

    pub(crate) unsafe fn wef_browser_set_size(browser: *mut wef_browser_t, width: i32, height: i32);

    pub(crate) unsafe fn wef_browser_load_url(cebrowserf: *mut wef_browser_t, url: *const c_char);

    pub(crate) unsafe fn wef_browser_can_go_forward(browser: *const wef_browser_t) -> bool;

    pub(crate) unsafe fn wef_browser_can_go_back(browser: *const wef_browser_t) -> bool;

    pub(crate) unsafe fn wef_browser_go_forward(browser: *mut wef_browser_t);

    pub(crate) unsafe fn wef_browser_go_back(browser: *mut wef_browser_t);

    pub(crate) unsafe fn wef_browser_reload(browser: *mut wef_browser_t);

    pub(crate) unsafe fn wef_browser_reload_ignore_cache(browser: *mut wef_browser_t);

    pub(crate) unsafe fn wef_browser_send_mouse_click_event(
        browser: *mut wef_browser_t,
        mouse_button_type: i32,
        mouse_up: bool,
        click_count: i32,
        modifiers: i32,
    );

    pub(crate) unsafe fn wef_browser_send_mouse_move_event(
        browser: *mut wef_browser_t,
        x: i32,
        y: i32,
        modifiers: i32,
    );

    pub(crate) unsafe fn wef_browser_send_mouse_wheel_event(
        browser: *mut wef_browser_t,
        delta_x: i32,
        delta_y: i32,
    );

    pub(crate) unsafe fn wef_browser_send_key_event(
        browser: *mut wef_browser_t,
        is_press: bool,
        key_code: i32,
        modifiers: i32,
    );

    pub(crate) unsafe fn wef_browser_send_char_event(browser: *mut wef_browser_t, ch: u16);

    pub(crate) unsafe fn wef_browser_ime_set_composition(
        browser: *mut wef_browser_t,
        text: *const c_char,
        cursor_begin: u32,
        cursor_end: u32,
    );

    pub(crate) unsafe fn wef_browser_ime_commit(browser: *mut wef_browser_t, text: *const c_char);

    pub(crate) unsafe fn wef_browser_get_main_frame(
        browser: *mut wef_browser_t,
    ) -> *mut wef_frame_t;

    pub(crate) unsafe fn wef_browser_get_focused_frame(
        browser: *mut wef_browser_t,
    ) -> *mut wef_frame_t;

    pub(crate) unsafe fn wef_browser_get_frame_by_name(
        browser: *mut wef_browser_t,
        name: *const c_char,
    ) -> *mut wef_frame_t;

    pub(crate) unsafe fn wef_browser_get_frame_by_identifier(
        browser: *mut wef_browser_t,
        id: *const c_char,
    ) -> *mut wef_frame_t;

    pub(crate) unsafe fn wef_browser_is_audio_muted(browser: *mut wef_browser_t) -> bool;

    pub(crate) unsafe fn wef_browser_set_audio_mute(browser: *mut wef_browser_t, mute: bool);

    pub(crate) unsafe fn wef_browser_find(
        browser: *mut wef_browser_t,
        search_text: *const c_char,
        forward: bool,
        match_case: bool,
        find_next: bool,
    );

    pub(crate) unsafe fn wef_browser_set_focus(browser: *mut wef_browser_t, focus: bool);

    pub(crate) unsafe fn wef_dirty_rects_len(dirty_rects: *const c_void) -> i32;

    pub(crate) unsafe fn wef_dirty_rects_get(
        dirty_rects: *const c_void,
        i: i32,
        rect: *mut Rect<i32>,
    );

    pub(crate) unsafe fn wef_frame_destroy(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_is_valid(frame: *mut wef_frame_t) -> bool;

    pub(crate) unsafe fn wef_frame_is_main(frame: *mut wef_frame_t) -> bool;

    pub(crate) unsafe fn wef_frame_name(
        frame: *mut wef_frame_t,
        userdata: *mut c_void,
        callback: extern "C" fn(*mut c_void, *const c_char),
    ) -> i32;

    pub(crate) unsafe fn wef_frame_identifier(
        frame: *mut wef_frame_t,
        userdata: *mut c_void,
        callback: extern "C" fn(*mut c_void, *const c_char),
    ) -> i32;

    pub(crate) unsafe fn wef_frame_get_url(
        frame: *mut wef_frame_t,
        userdata: *mut c_void,
        callback: extern "C" fn(*mut c_void, *const c_char),
    ) -> i32;

    pub(crate) unsafe fn wef_frame_load_url(frame: *mut wef_frame_t, url: *const c_char);

    pub(crate) unsafe fn wef_frame_parent(frame: *mut wef_frame_t) -> *mut c_void;

    pub(crate) unsafe fn wef_frame_undo(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_redo(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_cut(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_copy(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_paste(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_paste_and_match_style(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_delete(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_select_all(frame: *mut wef_frame_t);

    pub(crate) unsafe fn wef_frame_execute_javascript(frame: *mut wef_frame_t, code: *const c_char);

    pub(crate) unsafe fn wef_file_dialog_callback_continue(
        callback: *mut wef_file_dialog_callback_t,
        file_paths: *const c_char,
    );

    pub(crate) unsafe fn wef_file_dialog_callback_cancel(callback: *mut wef_file_dialog_callback_t);

    pub(crate) unsafe fn wef_file_dialog_callback_destroy(
        callback: *mut wef_file_dialog_callback_t,
    );

    pub(crate) unsafe fn wef_cursor_info_hotspot(
        info: *const wef_cursor_info_t,
        point: *mut Point<i32>,
    );

    pub(crate) unsafe fn wef_cursor_info_image_scale_factor(info: *const wef_cursor_info_t) -> f32;

    pub(crate) unsafe fn wef_cursor_info_buffer(info: *const wef_cursor_info_t) -> *const c_void;

    pub(crate) unsafe fn wef_cursor_info_size(info: *const wef_cursor_info_t, size: *mut Size<i32>);

    pub(crate) unsafe fn wef_js_dialog_callback_continue(
        callback: *mut wef_js_dialog_callback_t,
        success: bool,
        user_input: *const c_char,
    );

    pub(crate) unsafe fn wef_js_dialog_callback_destroy(callback: *mut wef_js_dialog_callback_t);

    pub(crate) unsafe fn wef_query_callback_success(
        callback: *mut wef_query_callback_t,
        response: *const c_char,
    );

    pub(crate) unsafe fn wef_query_callback_failure(
        callback: *mut wef_query_callback_t,
        error: *const c_char,
    );

    pub(crate) unsafe fn wef_query_callback_destroy(callback: *mut wef_query_callback_t);
}
