use std::os::raw::c_void;

use crate::{Error, ffi::*};

/// Scoped helper for loading and unloading the CEF framework library at
/// runtime from the expected location in the app bundle.
#[derive(Debug)]
pub struct FrameworkLoader(*mut c_void);

impl Drop for FrameworkLoader {
    fn drop(&mut self) {
        unsafe { wef_unload_library(self.0) };
    }
}

impl FrameworkLoader {
    fn load(helper: bool) -> Result<Self, Error> {
        unsafe {
            let loader = wef_load_library(helper);
            if loader.is_null() {
                return Err(Error::LoadLibrary);
            }
            Ok(Self(loader))
        }
    }

    /// Load the CEF framework in the main process from the expected app
    /// bundle location relative to the executable.
    pub fn load_in_main() -> Result<Self, Error> {
        Self::load(false)
    }

    /// Load the CEF framework in the helper process from the expected app
    /// bundle location relative to the executable.
    pub fn load_in_helper() -> Result<Self, Error> {
        Self::load(true)
    }
}
