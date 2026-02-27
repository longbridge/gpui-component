use std::ops::Range;

#[cfg(not(target_arch = "wasm32"))]
use tree_sitter::Node;
#[cfg(not(target_arch = "wasm32"))]
pub use tree_sitter::Tree;

#[cfg(target_arch = "wasm32")]
/// Stub type for tree-sitter Tree on WASM (tree-sitter not available).
pub struct Tree;

#[cfg(not(target_arch = "wasm32"))]
/// Minimum line span for a node to be considered foldable.
const MIN_FOLD_LINES: usize = 2;

/// A fold range representing a foldable code region.
///
/// The fold range spans from start_line to end_line (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoldRange {
    /// Start line (inclusive)
    pub start_line: usize,
    /// End line (inclusive)
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

// ==================== Native Implementation (with tree-sitter) ====================

#[cfg(not(target_arch = "wasm32"))]
/// Check if a named node qualifies as a fold candidate.
///
/// Uses a structural heuristic: any **named** node spanning ≥ MIN_FOLD_LINES
/// is foldable. tree-sitter already parses code into semantic units (functions,
/// classes, blocks, etc.), so named nodes naturally correspond to meaningful
/// foldable regions across all languages without a per-language node-type list.
fn is_foldable_node(node: &Node) -> bool {
    // Skip root node (e.g. `source_file`) and unnamed tokens
    if !node.is_named() || node.parent().is_none() {
        return false;
    }

    let start = node.start_position().row;
    let end = node.end_position().row;
    end.saturating_sub(start) >= MIN_FOLD_LINES
}

#[cfg(not(target_arch = "wasm32"))]
/// Extract fold ranges from a tree-sitter syntax tree (full traversal).
pub fn extract_fold_ranges(tree: &Tree) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    collect_foldable_nodes(tree.root_node(), &mut ranges);

    ranges.sort_by_key(|r| r.start_line);
    ranges.dedup_by_key(|r| r.start_line);
    ranges
}

#[cfg(not(target_arch = "wasm32"))]
/// Extract fold ranges only within a byte range (for incremental updates after edits).
///
/// Skips subtrees entirely outside the range, making it O(nodes in range)
/// instead of O(all nodes in tree).
pub fn extract_fold_ranges_in_range(tree: &Tree, byte_range: Range<usize>) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    collect_foldable_nodes_in_range(tree.root_node(), &byte_range, &mut ranges);

    ranges.sort_by_key(|r| r.start_line);
    ranges.dedup_by_key(|r| r.start_line);
    ranges
}

#[cfg(not(target_arch = "wasm32"))]
/// Recursively collect foldable nodes, skipping subtrees outside byte_range.
fn collect_foldable_nodes_in_range(
    node: Node,
    byte_range: &Range<usize>,
    ranges: &mut Vec<FoldRange>,
) {
    if node.end_byte() <= byte_range.start || node.start_byte() >= byte_range.end {
        return;
    }

    if is_foldable_node(&node) {
        ranges.push(FoldRange {
            start_line: node.start_position().row,
            end_line: node.end_position().row,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_foldable_nodes_in_range(child, byte_range, ranges);
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Recursively collect foldable nodes from the syntax tree (full traversal).
fn collect_foldable_nodes(node: Node, ranges: &mut Vec<FoldRange>) {
    if is_foldable_node(&node) {
        ranges.push(FoldRange {
            start_line: node.start_position().row,
            end_line: node.end_position().row,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_foldable_nodes(child, ranges);
    }
}

// ==================== WASM Stub Implementation ====================

#[cfg(target_arch = "wasm32")]
/// Extract fold ranges - WASM stub (returns empty, no tree-sitter).
pub fn extract_fold_ranges(_tree: &Tree) -> Vec<FoldRange> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
/// Extract fold ranges in range - WASM stub (returns empty, no tree-sitter).
pub fn extract_fold_ranges_in_range(_tree: &Tree, _byte_range: Range<usize>) -> Vec<FoldRange> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_range_ordering() {
        let mut ranges = vec![
            FoldRange {
                start_line: 10,
                end_line: 20,
            },
            FoldRange {
                start_line: 5,
                end_line: 15,
            },
            FoldRange {
                start_line: 5,
                end_line: 15,
            },
            FoldRange {
                start_line: 1,
                end_line: 30,
            },
        ];

        ranges.sort_by_key(|r| r.start_line);
        ranges.dedup_by_key(|r| r.start_line);

        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0].start_line, 1);
        assert_eq!(ranges[1].start_line, 5);
        assert_eq!(ranges[2].start_line, 10);
    }
}
