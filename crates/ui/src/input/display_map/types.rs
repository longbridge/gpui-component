/// Core coordinate types for the display mapping system.
///
/// This module defines the three coordinate systems used in text display:
/// - Buffer coordinates (logical lines and columns in the actual text)
/// - Wrap coordinates (soft-wrapped visual rows)
/// - Display coordinates (final visible rows after folding)

/// Position in the buffer (logical text).
///
/// - `line`: 0-based logical line number (split by `\n`)
/// - `col`: 0-based column offset (byte or char index, consistent with existing implementation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferPos {
    pub line: usize,
    pub col: usize,
}

impl BufferPos {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    pub fn zero() -> Self {
        Self { line: 0, col: 0 }
    }
}

/// Position after soft-wrapping but before folding (internal representation).
///
/// - `row`: 0-based wrap row (visual line after soft-wrap)
/// - `col`: 0-based visual column
///
/// Note: This is an internal type and should not be exposed to Editor/Input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct WrapPos {
    pub row: usize,
    pub col: usize,
}

impl WrapPos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn zero() -> Self {
        Self { row: 0, col: 0 }
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

    pub fn zero() -> Self {
        Self { row: 0, col: 0 }
    }
}

// FoldRange is now defined in crate::highlighter::folding
// and re-exported from this module via mod.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pos() {
        let pos = BufferPos::new(10, 5);
        assert_eq!(pos.line, 10);
        assert_eq!(pos.col, 5);

        let zero = BufferPos::zero();
        assert_eq!(zero.line, 0);
        assert_eq!(zero.col, 0);
    }
}
