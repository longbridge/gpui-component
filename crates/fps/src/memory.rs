//! How much memory this process is responsible for, as opposed to how many
//! pages it happens to have mapped.
//!
//! `sysinfo` reports the resident set, and on every platform that counts the
//! read-only pages of every shared library the process maps. The graphics stack
//! alone is most of it: a window on a machine with a proprietary driver *and*
//! Mesa loaded maps a shader compiler, a GL core and a handful of driver
//! libraries, which is a few hundred megabytes of code the process neither
//! allocated nor can release, and which every other window on the machine is
//! mapping at the same time. A HUD that reports the sum tells the reader
//! almost nothing about their own application — the number moves when a
//! *different* program starts.
//!
//! So each platform reads the counter its own activity monitor shows: the
//! private, dirty memory that exists because this process is running and would
//! come back if it exited. The three are not byte-identical — they cannot be,
//! since the platforms do not account for memory the same way — but they answer
//! the same question, and each matches what the reader would see if they went
//! looking in Activity Monitor, Task Manager or `top`.

#[cfg_attr(target_os = "macos", path = "memory/macos.rs")]
#[cfg_attr(target_os = "windows", path = "memory/windows.rs")]
#[cfg_attr(target_os = "linux", path = "memory/linux.rs")]
#[cfg_attr(
    not(any(target_os = "macos", target_os = "windows", target_os = "linux")),
    path = "memory/unsupported.rs"
)]
mod platform;

/// Samples this process' private memory.
///
/// [`new`] returns `None` on a platform with no such counter, and the caller
/// then falls back to the resident set — a worse number, but a present one.
///
/// [`new`]: MemoryProbe::new
pub(crate) struct MemoryProbe(platform::Probe);

impl MemoryProbe {
    pub(crate) fn new() -> Option<Self> {
        platform::Probe::new().map(Self)
    }

    /// Private memory, in bytes, or `None` for a reading that is momentarily
    /// unavailable.
    pub(crate) fn sample(&mut self) -> Option<u64> {
        self.0.sample()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a probe exists is the platform's answer; what must hold is that
    /// one which does exist reports bytes rather than the kibibytes or pages
    /// its counter is published in. A test process running the rest of this
    /// suite owns megabytes, so a reading under one is a unit that never got
    /// converted.
    #[test]
    fn a_reading_is_in_bytes() {
        let Some(mut probe) = MemoryProbe::new() else {
            return;
        };
        let Some(bytes) = probe.sample() else {
            return;
        };

        assert!(
            bytes >= 1024 * 1024,
            "{bytes} is too small to be this process' private memory in bytes"
        );
    }
}
