//! Cross-platform async utilities for both native and WASM targets.

// For native targets, re-export smol's primitives
#[cfg(not(target_arch = "wasm32"))]
pub use smol::channel::{Sender, Receiver, unbounded};

// For WASM targets, use async-channel
#[cfg(target_arch = "wasm32")]
pub use async_channel::{Sender, Receiver, unbounded};
