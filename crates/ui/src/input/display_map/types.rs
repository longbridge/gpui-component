/// Core coordinate types for the display mapping system.
///
/// - Buffer coordinates (logical lines and columns in the actual text)
/// - Wrap coordinates (soft-wrapped visual rows)
/// - Display coordinates (final visible rows after folding)

/// Position in the buffer (logical text).
///
/// - `line`: 0-based logical line number (split by `\n`)
/// - `col`: 0-based column offset (byte offset)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferPos {
    pub line: usize,
    pub col: usize,
}

impl BufferPos {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// Position after soft-wrapping but before folding (internal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WrapPos {
    pub row: usize,
    pub col: usize,
}

impl WrapPos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Final display position (after soft-wrapping and folding).
///
/// - `row`: 0-based display row (final visible row)
/// - `col`: 0-based display column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DisplayPos {
    pub row: usize,
    pub col: usize,
}

impl DisplayPos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}
