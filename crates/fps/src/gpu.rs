//! How much of the GPU this process is using, on the platforms that attribute
//! it per process without a vendor SDK or elevated privileges.
//!
//! It is this process' share and not the device's, to match the CPU and memory
//! beside it: a HUD that counted the compositor and every other window would
//! move for reasons the application cannot act on. Each platform reaches it
//! differently, and where none of them can, the reading is simply absent.

#[cfg_attr(target_os = "macos", path = "gpu/macos.rs")]
#[cfg_attr(target_os = "windows", path = "gpu/windows.rs")]
#[cfg_attr(target_os = "linux", path = "gpu/linux.rs")]
#[cfg_attr(
    not(any(target_os = "macos", target_os = "windows", target_os = "linux")),
    path = "gpu/unsupported.rs"
)]
mod platform;

/// Samples GPU utilization.
///
/// Every backend answers the same two questions, and both can say no. [`new`]
/// returns `None` when this platform has no counter to read at all, or when the
/// machine does not publish one — a driver on Linux that omits it, a GPU with
/// no statistics — and the HUD then leaves the row out entirely rather than
/// showing an empty slot. [`sample`] returns `None` for a reading that is only
/// momentarily unavailable, leaving the last one on screen.
///
/// [`new`]: GpuProbe::new
/// [`sample`]: GpuProbe::sample
pub(crate) struct GpuProbe(platform::Probe);

impl GpuProbe {
    pub(crate) fn new() -> Option<Self> {
        platform::Probe::new().map(Self)
    }

    /// The share of the wall clock the GPU spent on this process since the
    /// previous call, in `0..=100`.
    ///
    /// The clamp matters: several engines can run at once, so the raw sum of
    /// their busy time can pass the wall clock it is divided by.
    pub(crate) fn sample(&mut self) -> Option<f32> {
        self.0.sample().map(|percent| percent.clamp(0., 100.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a probe exists at all is the platform's answer, and a headless
    /// CI machine may well have no accelerator to report; what must hold is
    /// that a probe which does exist reports a percentage rather than a raw
    /// counter or a fraction.
    #[test]
    fn a_reading_is_a_percentage() {
        let Some(mut probe) = GpuProbe::new() else {
            return;
        };

        if let Some(percent) = probe.sample() {
            assert!(
                (0. ..=100.).contains(&percent),
                "{percent} is not a percentage"
            );
        }
    }
}
