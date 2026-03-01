// Stub implementation of errno for WASM
// In WASM, we don't have a traditional errno system

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Errno(pub i32);

impl Errno {
    pub fn last() -> Self {
        Errno(0)
    }

    pub fn clear() {}

    pub fn from_i32(errno: i32) -> Self {
        Errno(errno)
    }
}

pub fn errno() -> Errno {
    Errno::last()
}

pub fn set_errno(Errno(_errno): Errno) {}
