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
mod text_wrapper;
mod types;
mod wrap_map;

// Re-export public API
pub use self::display_map::DisplayMap;
pub use self::types::{BufferPos, DisplayPos, FoldRange};

// Re-export for gradual migration (TODO: remove after full migration)
pub use self::fold_map::FoldMap;
pub use self::wrap_map::WrapMap;
