// Diagnostics module - works on all platforms (no tree-sitter dependency)
mod diagnostics;
pub use diagnostics::*;

// Native implementation with full tree-sitter support
#[cfg(feature = "tree-sitter")]
mod highlighter;
#[cfg(feature = "tree-sitter")]
mod languages;
#[cfg(feature = "tree-sitter")]
mod registry;

#[cfg(feature = "tree-sitter")]
pub use highlighter::*;
#[cfg(feature = "tree-sitter")]
pub use languages::*;
#[cfg(feature = "tree-sitter")]
pub use registry::*;

// WASM stub implementation (no tree-sitter support or disabled)
#[cfg(not(feature = "tree-sitter"))]
mod wasm_stub;
#[cfg(not(feature = "tree-sitter"))]
pub use wasm_stub::*;
