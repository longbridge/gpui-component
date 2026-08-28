//! Reads `ri_phys_footprint` through `proc_pid_rusage`, the counter behind
//! Activity Monitor's Memory column and the one the kernel judges a process
//! against under memory pressure — the number that decides whether this
//! application is the one that gets jetsammed.
//!
//! It is the dirty and compressed memory the task owns, so unlike its resident
//! size it does not count the clean pages of the frameworks, the Metal stack
//! and the drivers it maps, which every other application on the machine maps
//! at the same time.
//!
//! `proc_pid_rusage` rather than `task_info(TASK_VM_INFO)`, which carries the
//! same footprint: the rusage flavors are a stable public interface with a
//! versioned layout, while `task_vm_info_data_t` is a Mach structure whose size
//! is the version, so reading it means hard-coding a layout that has grown
//! across releases.

use std::mem::MaybeUninit;

pub(super) struct Probe {
    pid: libc::c_int,
}

impl Probe {
    pub(super) fn new() -> Option<Self> {
        let probe = Self {
            pid: std::process::id() as libc::c_int,
        };
        // Doubles as the support check, though there is no macOS this call is
        // absent from; it is here so a refusal shows up as a missing reading
        // rather than as a HUD stuck at zero.
        probe.read()?;
        Some(probe)
    }

    pub(super) fn sample(&mut self) -> Option<u64> {
        self.read()
    }

    /// `V2` is the oldest flavor carrying `ri_phys_footprint`, so it is the one
    /// asked for: a later flavor would only add fields this does not read while
    /// narrowing the range of systems that answer.
    fn read(&self) -> Option<u64> {
        let mut info = MaybeUninit::<libc::rusage_info_v2>::uninit();
        // SAFETY: `info` is a live buffer of exactly the layout `RUSAGE_INFO_V2`
        // names, which is the only thing the call writes through the pointer.
        let status = unsafe {
            libc::proc_pid_rusage(
                self.pid,
                libc::RUSAGE_INFO_V2,
                info.as_mut_ptr().cast::<libc::rusage_info_t>(),
            )
        };
        if status != 0 {
            return None;
        }
        // SAFETY: a zero return means the call filled the buffer.
        Some(unsafe { info.assume_init() }.ri_phys_footprint)
    }
}
