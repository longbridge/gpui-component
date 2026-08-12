use std::ops::Range;

use ropey::Rope;

/// Supplies parser-independent fold candidates to the display projection.
pub trait FoldCandidateProvider {
    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange>;

    fn fold_ranges_for_edit(&self, range: Range<usize>, text: &Rope) -> Vec<FoldRange> {
        let _ = range;
        self.fold_ranges(text)
    }
}

/// A foldable line range supplied by an editor highlighter or another source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line: usize,
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
