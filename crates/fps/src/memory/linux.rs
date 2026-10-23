//! Reads `RssAnon` out of `/proc/self/status`: the resident anonymous memory of
//! this process — its heap, its thread stacks and every private mapping — and
//! none of the files it maps.
//!
//! `RssAnon` rather than the `Private_Dirty` of `/proc/self/smaps_rollup`, which
//! is the closer analogue of what macOS and Windows report here. `smaps_rollup`
//! is computed by walking every mapping in the address space under its lock,
//! and on a windowed application — a few hundred mappings once the graphics
//! stack is up — that measures ~425us a read against ~5us for `status`. The
//! difference between the two counters is the process' private *file* pages:
//! relocations, `.data`, the GOT, single-digit megabytes that do not move. A
//! performance HUD should not take the address space lock twice a second to
//! account for them — least of all one whose whole job is to not perturb what
//! it measures.

use std::fs;

/// Per-process counters, including the resident set split by kind. Reading it
/// formats counters the kernel already holds, with no walk of the address space
/// behind it.
const STATUS: &str = "/proc/self/status";

/// The line carrying resident anonymous memory, as in `RssAnon:    118016 kB`.
/// Split out of `VmRSS` in 4.5, which is old enough not to need a fallback.
const ANONYMOUS: &str = "RssAnon:";

pub(super) struct Probe {
    _private: (),
}

impl Probe {
    pub(super) fn new() -> Option<Self> {
        // Doubles as the support check: a kernel that does not publish the
        // line leaves the HUD on the resident set rather than at zero.
        read()?;
        Some(Self { _private: () })
    }

    pub(super) fn sample(&mut self) -> Option<u64> {
        read()
    }
}

/// The kernel publishes the value in kibibytes, and the unit is part of the
/// line; it is parsed as a fixed `kB` rather than read back, since every
/// counter in this file is published in it.
fn read() -> Option<u64> {
    let status = fs::read_to_string(STATUS).ok()?;
    let value = status
        .lines()
        .find_map(|line| line.strip_prefix(ANONYMOUS))?;
    let kibibytes: u64 = value.split_whitespace().next()?.parse().ok()?;
    Some(kibibytes * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the counter: it leaves out the file pages that make
    /// the resident set say more about the machine's graphics stack than about
    /// the application. On Linux this is an identity — `RssAnon` is one of the
    /// two halves `VmRSS` is the sum of — so it can be asserted rather than
    /// merely expected.
    #[test]
    fn anonymous_memory_is_a_part_of_the_resident_set() {
        let Some(anonymous) = read() else {
            return;
        };
        let status = fs::read_to_string(STATUS).expect("`read` just parsed this file");
        let resident: u64 = status
            .lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .and_then(|value| value.split_whitespace().next()?.parse().ok())
            .map(|kibibytes: u64| kibibytes * 1024)
            .expect("every kernel publishes VmRSS");

        assert!(anonymous > 0, "a live process owns anonymous memory");
        assert!(
            anonymous <= resident,
            "{anonymous} bytes anonymous cannot exceed {resident} bytes resident"
        );
    }
}
