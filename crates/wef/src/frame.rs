use std::{
    ffi::{CStr, c_char, c_void},
    fmt,
};

use serde::Serialize;

use crate::ffi::*;

/// A frame in the browser window.
pub struct Frame(pub(crate) *mut wef_frame_t);

unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("name", &self.name())
            .field("identifier", &self.identifier())
            .field("url", &self.url())
            .finish()
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { wef_frame_destroy(self.0) };
    }
}

impl Frame {
    /// Returns `true` if this object is currently attached to a valid frame.
    pub fn is_valid(&self) -> bool {
        unsafe { wef_frame_is_valid(self.0) }
    }

    /// Returns `true` if this is the main (top-level) frame.
    pub fn is_main(&self) -> bool {
        unsafe { wef_frame_is_main(self.0) }
    }

    /// Returns the name for this frame.
    ///
    /// If the frame has an assigned name (for example, set via the iframe
    /// "name" attribute) then that value will be returned. Otherwise a
    /// unique name will be constructed based on the frame parent hierarchy.
    ///
    /// Returns `None` if the frame is main (top-level).
    pub fn name(&self) -> Option<String> {
        let mut name = String::new();
        unsafe { wef_frame_name(self.0, &mut name as *mut _ as _, get_string_callback) };
        (!name.is_empty()).then_some(name)
    }

    /// Returns the globally unique identifier for this frame or `None` if the
    /// underlying frame does not yet exist.
    pub fn identifier(&self) -> Option<String> {
        let mut id = String::new();
        unsafe { wef_frame_identifier(self.0, &mut id as *mut _ as _, get_string_callback) };
        (!id.is_empty()).then_some(id)
    }

    /// Returns the URL currently loaded in this frame.
    pub fn url(&self) -> String {
        let mut url = String::new();
        unsafe { wef_frame_get_url(self.0, &mut url as *mut _ as _, get_string_callback) };
        url
    }

    /// Loads the specified URL in this frame.
    pub fn load_url(&self, url: &str) {
        let c_url = std::ffi::CString::new(url).unwrap();
        unsafe { wef_frame_load_url(self.0, c_url.as_ptr()) };
    }

    /// Returns the parent of this frame or `None` if this is the main
    /// (top-level) frame.
    pub fn parent(&self) -> Option<Frame> {
        let frame = unsafe { wef_frame_parent(self.0) };
        if !frame.is_null() {
            Some(Frame(frame))
        } else {
            None
        }
    }

    /// Execute undo in this frame.
    pub fn undo(&self) {
        unsafe { wef_frame_undo(self.0) };
    }

    /// Execute redo in this frame.
    pub fn redo(&self) {
        unsafe { wef_frame_redo(self.0) };
    }

    /// Execute cut in this frame.
    pub fn cut(&self) {
        unsafe { wef_frame_cut(self.0) };
    }

    /// Execute copy in this frame.
    pub fn copy(&self) {
        unsafe { wef_frame_copy(self.0) };
    }

    /// Execute paste in this frame.
    pub fn paste(&self) {
        unsafe { wef_frame_paste(self.0) };
    }

    /// Execute paste and match style in this frame.
    pub fn paste_and_match_style(&self) {
        unsafe { wef_frame_paste_and_match_style(self.0) };
    }

    /// Execute delete in this frame.
    pub fn delete(&self) {
        unsafe { wef_frame_delete(self.0) };
    }

    /// Execute select all in this frame.
    pub fn select_all(&self) {
        unsafe { wef_frame_select_all(self.0) };
    }

    /// Execute javascript in this frame.
    pub fn execute_javascript(&self, script: &str) {
        if script.is_empty() {
            return;
        }
        let c_script = std::ffi::CString::new(script).unwrap();
        unsafe { wef_frame_execute_javascript(self.0, c_script.as_ptr()) };
    }

    /// Emits a message to the JavaScript side.
    pub fn emit(&self, message: impl Serialize) {
        let Ok(message) = serde_json::to_string(&message) else {
            return;
        };
        self.execute_javascript(&format!("window.jsBridge.__internal.emit({})", message));
    }
}

extern "C" fn get_string_callback(output: *mut c_void, value: *const c_char) {
    unsafe {
        *(output as *mut String) = CStr::from_ptr(value)
            .to_str()
            .unwrap_or_default()
            .to_string();
    }
}
