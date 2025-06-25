use std::ffi::CString;

use serde_json::Value;

use crate::{ffi::*, func_registry::CallFunctionError};

pub(crate) struct QueryCallback(*mut wef_query_callback_t);

unsafe impl Send for QueryCallback {}
unsafe impl Sync for QueryCallback {}

impl Drop for QueryCallback {
    fn drop(&mut self) {
        unsafe { wef_query_callback_destroy(self.0) };
    }
}

impl QueryCallback {
    pub(crate) fn new(callback: *mut wef_query_callback_t) -> Self {
        Self(callback)
    }

    pub(crate) fn result(self, res: Result<Value, CallFunctionError>) {
        match res {
            Ok(resp) => self.success(resp),
            Err(err) => self.failure(err),
        }
    }

    fn success(self, resp: Value) {
        let resp = CString::new(serde_json::to_string(&resp).unwrap_or_default()).unwrap();
        unsafe { wef_query_callback_success(self.0, resp.as_ptr()) }
    }

    fn failure(self, err: CallFunctionError) {
        let err = CString::new(err.to_string()).unwrap();
        unsafe { wef_query_callback_failure(self.0, err.as_ptr()) }
    }
}
