use std::{
    ffi::{CString, c_char},
    os::raw::c_void,
};

use crate::{Error, ffi::*};

/// The sandbox is used to restrict sub-processes (renderer, GPU, etc) from
/// directly accessing system resources.
///
/// This helps to protect the user from untrusted and potentially malicious Web
/// content.
#[derive(Debug)]
pub struct SandboxContext(*mut c_void);

impl SandboxContext {
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let args: Vec<CString> = std::env::args()
                .filter_map(|arg| CString::new(arg).ok())
                .collect();
            let c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();

            let context = wef_sandbox_context_create(c_args.as_ptr(), args.len() as i32);
            if context.is_null() {
                return Err(Error::SandboxContextCreate);
            }
            Ok(SandboxContext(context))
        }
    }
}

impl Drop for SandboxContext {
    fn drop(&mut self) {
        unsafe { wef_sandbox_context_destroy(self.0) };
    }
}
