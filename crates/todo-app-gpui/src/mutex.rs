use std::ffi::CString;

use windows::{
    core::{Error, Result, PCSTR},
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE},
        System::Threading::CreateMutexA,
    },
};

pub struct Mutex(HANDLE);

unsafe impl Send for Mutex {}
unsafe impl Sync for Mutex {}

impl Drop for Mutex {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

impl Mutex {
    pub fn try_lock<S: AsRef<str>>(key: S, owner: bool) -> Result<Self> {
        let key = CString::new(key.as_ref()).unwrap();
        let handle = unsafe { CreateMutexA(None, owner, PCSTR(key.as_ptr() as _)) }?;
        unsafe {
            let res = GetLastError().to_hresult();
            if res.is_err() {
                let _ = CloseHandle(handle);
                return Err(Error::new(windows::core::HRESULT(handle.0 as _), ""));
            }
        }

        if handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(Error::new(windows::core::HRESULT(handle.0 as _), ""));
        }
        Ok(Self(handle))
    }
}
