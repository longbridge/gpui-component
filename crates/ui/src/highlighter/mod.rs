// Diagnostics module - works on all platforms (no tree-sitter dependency)
mod diagnostics;
pub use diagnostics::*;

// Native implementation with full tree-sitter support
#[cfg(not(target_family = "wasm"))]
mod highlighter;
#[cfg(not(target_family = "wasm"))]
mod languages;
#[cfg(not(target_family = "wasm"))]
mod registry;

#[cfg(not(target_family = "wasm"))]
pub use highlighter::*;
#[cfg(not(target_family = "wasm"))]
pub use languages::*;
#[cfg(not(target_family = "wasm"))]
pub use registry::*;

// WASM stub implementation (no tree-sitter support)
#[cfg(target_family = "wasm")]
mod wasm_stub;
#[cfg(target_family = "wasm")]
pub use wasm_stub::*;
