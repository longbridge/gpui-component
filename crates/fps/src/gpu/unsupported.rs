//! Platforms with no GPU counter reachable from an ordinary process.
//! Construction always fails, so the HUD leaves the reading out.

pub(super) struct Probe;

impl Probe {
    pub(super) fn new() -> Option<Self> {
        None
    }

    pub(super) fn sample(&mut self) -> Option<f32> {
        None
    }
}
