//! Platforms that publish no private-memory counter to an ordinary process.
//! Construction always fails, so the HUD falls back to the resident set.

pub(super) struct Probe;

impl Probe {
    pub(super) fn new() -> Option<Self> {
        None
    }

    pub(super) fn sample(&mut self) -> Option<u64> {
        None
    }
}
