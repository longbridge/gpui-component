use std::ffi::CString;

use mime::Mime;
use num_enum::TryFromPrimitive;

use crate::ffi::*;

/// Supported file dialog modes.
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum FileDialogMode {
    /// Requires that the file exists before allowing the user to pick it.
    Open = 0,
    /// Like Open, but allows picking multiple files to open.
    OpenMultiple = 1,
    /// Like Open, but selects a folder to open.
    OpenFolder = 2,
    /// Allows picking a nonexistent file, and prompts to overwrite if the file
    /// already exists.
    Save = 3,
}

#[derive(Debug)]
pub enum AcceptFilter<'a> {
    Mime(Mime),
    Extension(&'a str),
}

/// Accept filter for file dialogs.
#[derive(Debug)]
pub struct Accept<'a> {
    /// Used to restrict the selectable file types.
    ///
    /// May be any combination of valid lower-cased MIME types (e.g. "text/*" or
    /// "image/*") and individual file extensions (e.g. ".txt" or ".png").
    pub filters: AcceptFilter<'a>,
    /// Provides the expansion of MIME types to file extensions.
    pub extensions: Option<&'a [&'a str]>,
    /// Provides the descriptions for MIME types
    ///
    /// For example, the "image/*" mime type might have extensions
    /// [".png", ".jpg", ".bmp", ...] and description "Image Files".
    pub description: Option<&'a str>,
}

/// File dialog callback.
pub struct FileDialogCallback(*mut wef_file_dialog_callback_t);

unsafe impl Send for FileDialogCallback {}
unsafe impl Sync for FileDialogCallback {}

impl Drop for FileDialogCallback {
    fn drop(&mut self) {
        unsafe { wef_file_dialog_callback_destroy(self.0) };
    }
}

impl FileDialogCallback {
    pub(crate) fn new(callback: *mut wef_file_dialog_callback_t) -> Self {
        FileDialogCallback(callback)
    }

    /// Continue the file dialog with the selected file paths.
    pub fn continue_(&self, file_paths: &[&str]) {
        let paths = CString::new(file_paths.join(";")).unwrap();
        unsafe { wef_file_dialog_callback_continue(self.0, paths.as_ptr()) };
    }

    /// Cancel the file dialog.
    pub fn cancel(&self) {
        unsafe { wef_file_dialog_callback_cancel(self.0) };
    }
}
