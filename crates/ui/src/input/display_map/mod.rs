/// Display mapping system for Editor/Input.
///
/// This module implements a layered display mapping architecture:
/// - **WrapMap**: Handles soft-wrapping (buffer → wrap rows)
/// - **FoldMap**: Handles folding (wrap rows → display rows)
/// - **DisplayMap**: Public facade for Editor/Input
///
/// The goal is to provide a clean, unified API where Editor only needs to know
/// about `BufferPos ↔ DisplayPos` mapping, without worrying about internal wrap/fold complexity.
mod display_map;
mod fold_map;
mod folding;
mod text_wrapper;
mod types;
mod wrap_map;

// Re-export public API
pub use self::display_map::DisplayMap;
pub(crate) use self::text_wrapper::{LineItem, LineLayout};
pub use self::types::{BufferPos, DisplayPos};

// Re-export FoldRange and extract_fold_ranges
pub use folding::{FoldRange, extract_fold_ranges};

