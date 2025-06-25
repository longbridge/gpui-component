use std::ffi::CString;

use crate::ffi::*;

/// Supported JS dialog types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum JsDialogType<'a> {
    Alert,
    Confirm,
    Prompt { default_prompt_text: &'a str },
}

/// Js dialog callback.
pub struct JsDialogCallback(*mut wef_js_dialog_callback_t);

unsafe impl Send for JsDialogCallback {}
unsafe impl Sync for JsDialogCallback {}

impl Drop for JsDialogCallback {
    fn drop(&mut self) {
        unsafe { wef_js_dialog_callback_destroy(self.0) }
    }
}

impl JsDialogCallback {
    pub(crate) fn new(callback: *mut wef_js_dialog_callback_t) -> Self {
        JsDialogCallback(callback)
    }

    /// Continue the JS dialog request.
    ///
    /// Set `success` to true if the OK button was pressed.
    /// The `user_input` value should be specified for prompt dialogs.
    pub fn continue_(&self, success: bool, user_input: Option<&str>) {
        unsafe {
            let user_input_cstr = user_input
                .and_then(|user_input| CString::new(user_input).ok())
                .unwrap_or_default();
            wef_js_dialog_callback_continue(self.0, success, user_input_cstr.as_ptr());
        }
    }
}
