use std::{ffi::CString, fmt};

use crate::{
    BrowserBuilder, Frame, KeyCode, KeyModifier, LogicalUnit, MouseButton, PhysicalUnit, Point,
    Size, ffi::*,
};

/// A browser instance.
pub struct Browser {
    pub(crate) wef_browser: *mut wef_browser_t,
}

impl fmt::Debug for Browser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Browser").finish()
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        unsafe { wef_browser_destroy(self.wef_browser) };
    }
}

impl Browser {
    /// Creates a new [`BrowserBuilder`] instance.
    pub fn builder() -> BrowserBuilder<()> {
        BrowserBuilder::new()
    }

    /// Closes the browser.
    ///
    /// When the browser has been closed, the callback
    /// [`crate::BrowserHandler::on_closed`] will be called.
    ///
    /// If the browser is not created or in creating state, this method does
    /// nothing.
    pub fn close(&self) {
        unsafe { wef_browser_close(self.wef_browser) }
    }

    /// Returns `true` if the browser is created.
    pub fn is_created(&self) -> bool {
        unsafe { wef_browser_is_created(self.wef_browser) }
    }

    /// Sets the size of the render target.
    pub fn resize(&self, sz: Size<PhysicalUnit<i32>>) {
        unsafe { wef_browser_set_size(self.wef_browser, sz.width.0.max(1), sz.height.0.max(1)) };
    }

    /// Loads a URL.
    pub fn load_url(&self, url: &str) {
        let c_url = CString::new(url).unwrap();
        unsafe { wef_browser_load_url(self.wef_browser, c_url.as_ptr()) };
    }

    /// Returns `true`` if the browser can navigate forwards.
    pub fn can_forward(&self) -> bool {
        unsafe { wef_browser_can_go_forward(self.wef_browser) }
    }

    /// Returns `true`` if the browser can navigate backwards.
    pub fn can_back(&self) -> bool {
        unsafe { wef_browser_can_go_back(self.wef_browser) }
    }

    /// Navigates forward in history.
    pub fn forward(&self) {
        unsafe { wef_browser_go_forward(self.wef_browser) };
    }

    /// Navigates back in history.
    pub fn back(&self) {
        unsafe { wef_browser_go_back(self.wef_browser) };
    }

    /// Reloads the current page.
    pub fn reload(&self) {
        unsafe { wef_browser_reload(self.wef_browser) };
    }

    /// Reloads the current page ignoring any cached data.
    pub fn reload_ignore_cache(&self) {
        unsafe { wef_browser_reload_ignore_cache(self.wef_browser) };
    }

    /// Sends a mouse click event.
    pub fn send_mouse_click_event(
        &self,
        mouse_button_type: MouseButton,
        mouse_up: bool,
        click_count: usize,
        modifiers: KeyModifier,
    ) {
        unsafe {
            wef_browser_send_mouse_click_event(
                self.wef_browser,
                mouse_button_type as i32,
                mouse_up,
                click_count as i32,
                modifiers.bits(),
            )
        };
    }

    /// Sends a mouse move event.
    pub fn send_mouse_move_event(&self, pt: Point<LogicalUnit<i32>>, modifiers: KeyModifier) {
        unsafe {
            wef_browser_send_mouse_move_event(self.wef_browser, pt.x.0, pt.y.0, modifiers.bits())
        };
    }

    /// Sends a mouse wheel event.
    pub fn send_mouse_wheel_event(&self, delta: Point<LogicalUnit<i32>>) {
        unsafe { wef_browser_send_mouse_wheel_event(self.wef_browser, delta.x.0, delta.y.0) };
    }

    /// Sends a key event.
    pub fn send_key_event(&self, is_down: bool, key_code: KeyCode, modifiers: KeyModifier) {
        unsafe {
            wef_browser_send_key_event(
                self.wef_browser,
                is_down,
                key_code as i32,
                modifiers.bits(),
            );
            if let Some(ch) = key_code.as_char() {
                wef_browser_send_char_event(self.wef_browser, ch);
            }
        }
    }

    /// Sends a character event.
    pub fn send_char_event(&self, ch: u16) {
        unsafe { wef_browser_send_char_event(self.wef_browser, ch) };
    }

    /// Sets the composition text for the IME (Input Method Editor).
    pub fn ime_set_composition(&self, text: &str, cursor_begin: usize, cursor_end: usize) {
        let c_text = CString::new(text).unwrap();
        unsafe {
            wef_browser_ime_set_composition(
                self.wef_browser,
                c_text.as_ptr(),
                cursor_begin as u32,
                cursor_end as u32,
            )
        };
    }

    /// Commits the composition text for the IME (Input Method Editor).
    pub fn ime_commit(&self, text: &str) {
        let c_text = CString::new(text).unwrap();
        unsafe { wef_browser_ime_commit(self.wef_browser, c_text.as_ptr()) };
    }

    /// Returns the main (top-level) frame for the browser.
    pub fn main_frame(&self) -> Option<Frame> {
        let frame = unsafe { wef_browser_get_main_frame(self.wef_browser) };
        (!frame.is_null()).then_some(Frame(frame))
    }

    /// Returns the focused frame for the browser.
    pub fn focused_frame(&self) -> Option<Frame> {
        let frame = unsafe { wef_browser_get_focused_frame(self.wef_browser) };
        (!frame.is_null()).then_some(Frame(frame))
    }

    /// Returns a frame by its name.
    pub fn frame_by_name(&self, name: &str) -> Option<Frame> {
        let c_name = CString::new(name).unwrap();
        let frame = unsafe { wef_browser_get_frame_by_name(self.wef_browser, c_name.as_ptr()) };
        (!frame.is_null()).then_some(Frame(frame))
    }

    /// Returns a frame by its identifier.
    pub fn frame_by_identifier(&self, id: &str) -> Option<Frame> {
        let c_id = CString::new(id).unwrap();
        let frame = unsafe { wef_browser_get_frame_by_identifier(self.wef_browser, c_id.as_ptr()) };
        (!frame.is_null()).then_some(Frame(frame))
    }

    /// Returns `true` if the browser's audio is muted.
    pub fn is_audio_muted(&self) -> bool {
        unsafe { wef_browser_is_audio_muted(self.wef_browser) }
    }

    /// Set whether the browser's audio is muted.
    pub fn set_audio_mute(&self, mute: bool) {
        unsafe { wef_browser_set_audio_mute(self.wef_browser, mute) };
    }

    /// Search for `searchText``.
    ///
    /// `forward`` indicates whether to search forward
    /// or backward within the page.
    /// `matchCase`` indicates whether the search should be case-sensitive.
    /// `findNext`` indicates whether this is the first request or a
    /// follow-up.
    ///
    /// The search will be restarted if `searchText` or `matchCase` change. The
    /// search will be stopped if `searchText` is empty.
    ///
    /// The find results will be reported via the
    /// [`crate::BrowserHandler::on_find_result`].
    pub fn find(&self, search_text: &str, forward: bool, match_case: bool, find_next: bool) {
        unsafe {
            let c_search_text = CString::new(search_text).unwrap();
            wef_browser_find(
                self.wef_browser,
                c_search_text.as_ptr(),
                forward,
                match_case,
                find_next,
            )
        };
    }

    /// Set whether the browser is focused.
    pub fn set_focus(&self, focus: bool) {
        unsafe { wef_browser_set_focus(self.wef_browser, focus) };
    }
}
