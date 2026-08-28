//! Reads `PrivateUsage` through `GetProcessMemoryInfo`: this process' private
//! commit, which is what Task Manager's Commit size column shows.
//!
//! It counts the pages the process itself asked the memory manager for, and not
//! the image pages its working set holds in common with every other process
//! mapping the same DLL — which on a windowed application is the whole graphics
//! stack. Private commit rather than the private *working set*, which is closer
//! to what the other platforms report but is only reachable per page through
//! `QueryWorkingSetEx`, a walk of the whole address space; the difference is
//! the process' own memory that has been paged out, which is memory it is still
//! responsible for.

use std::mem::size_of;

use windows::Win32::System::{
    ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX},
    Threading::GetCurrentProcess,
};

pub(super) struct Probe {
    _private: (),
}

impl Probe {
    pub(super) fn new() -> Option<Self> {
        // Doubles as the support check: without `PROCESS_QUERY_INFORMATION` the
        // call fails, and the HUD stays on the resident set rather than at zero.
        read()?;
        Some(Self { _private: () })
    }

    pub(super) fn sample(&mut self) -> Option<u64> {
        read()
    }
}

/// The call takes the shorter `PROCESS_MEMORY_COUNTERS`, and `cb` is what tells
/// it the buffer is in fact the longer structure that carries `PrivateUsage`.
/// The two share a prefix by definition, so the cast is the documented way to
/// ask for the extended counters.
fn read() -> Option<u64> {
    let size = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: size,
        ..Default::default()
    };
    // SAFETY: the pseudo handle from `GetCurrentProcess` needs no release, and
    // the buffer is live and described by the size passed alongside it.
    let read = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            std::ptr::from_mut(&mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
    };
    read.ok()?;
    Some(counters.PrivateUsage as u64)
}
