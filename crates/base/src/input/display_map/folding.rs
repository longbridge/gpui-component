use std::ops::Range;

use ropey::Rope;

use crate::input::InputHighlighter;

/// A foldable line range supplied by an editor highlighter or another source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line: usize,
}

pub(super) fn extract_fold_ranges_in_range(
    source: &dyn InputHighlighter,
    byte_range: Range<usize>,
    text: &Rope,
) -> Vec<FoldRange> {
    source.fold_ranges_for_edit(byte_range, text)
}

impl FoldRange {
    pub fn new(start_line: usize, end_line: usize) -> Self {
        assert!(
            start_line <= end_line,
            "fold start_line must be <= end_line"
        );
        Self {
            start_line,
            end_line,
        }
    }
}
