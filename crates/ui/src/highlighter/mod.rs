// Diagnostics module - works on all platforms (no tree-sitter dependency)
mod diagnostics;
pub use diagnostics::*;

// Native implementation with full tree-sitter support
#[cfg(not(target_arch = "wasm32"))]
mod highlighter;
#[cfg(not(target_arch = "wasm32"))]
mod languages;
#[cfg(not(target_arch = "wasm32"))]
mod registry;

#[cfg(not(target_arch = "wasm32"))]
pub use highlighter::*;
#[cfg(not(target_arch = "wasm32"))]
pub use languages::*;
#[cfg(not(target_arch = "wasm32"))]
pub use registry::*;

// WASM stub implementation (no tree-sitter support)
#[cfg(target_arch = "wasm32")]
mod wasm_stub;
#[cfg(target_arch = "wasm32")]
pub use wasm_stub::*;
