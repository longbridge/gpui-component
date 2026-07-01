// Diagnostics module - works on all platforms (no tree-sitter dependency)
mod diagnostics;
pub use diagnostics::*;

// Native implementation with full tree-sitter support
#[cfg(all(not(target_family = "wasm"), feature = "tree-sitter"))]
mod highlighter;
#[cfg(all(not(target_family = "wasm"), feature = "tree-sitter"))]
mod languages;
#[cfg(all(not(target_family = "wasm"), feature = "tree-sitter"))]
mod registry;

#[cfg(all(not(target_family = "wasm"), feature = "tree-sitter"))]
pub use highlighter::*;
#[cfg(all(not(target_family = "wasm"), feature = "tree-sitter"))]
pub use languages::*;
#[cfg(all(not(target_family = "wasm"), feature = "tree-sitter"))]
pub use registry::*;

// WASM stub implementation (no tree-sitter support or disabled)
#[cfg(any(target_family = "wasm", not(feature = "tree-sitter")))]
mod wasm_stub;
#[cfg(any(target_family = "wasm", not(feature = "tree-sitter")))]
pub use wasm_stub::*;
