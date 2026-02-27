//! Cross-platform async utilities for both native and WASM targets.

// For native targets, re-export smol's primitives
#[cfg(not(target_arch = "wasm32"))]
pub use smol::{Timer, stream, channel::{Sender, Receiver, bounded, unbounded}};

// For WASM targets, use async-channel and futures
#[cfg(target_arch = "wasm32")]
pub use async_channel::{Sender, Receiver, bounded, unbounded};

#[cfg(target_arch = "wasm32")]
pub use futures::stream;
