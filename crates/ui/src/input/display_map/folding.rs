use std::collections::HashSet;
use std::ops::Range;
use tree_sitter::{Node, Tree};

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

/// Foldable node types in tree-sitter syntax trees.
/// These types typically represent code blocks or structures.
/// We focus on semantic, top-level constructs to avoid over-folding.
const FOLDABLE_NODE_TYPES: &[&str] = &[
    // Functions and methods (high-level definitions)
    "function_definition",
    "function_declaration",
    "function_item",
    "method_definition",
    "method_declaration",

    // Classes, structs, traits (type definitions)
    "class_definition",
    "class_declaration",
    "class_body",
    "struct_item",
    "impl_item",
    "trait_item",
    "enum_item",
    "interface_declaration",

    // Modules and namespaces
    "module",
    "mod_item",
    "namespace",

    // Control flow (only multi-line constructs)
    "match_expression",
    "switch_statement",
    "try_statement",
    "catch_clause",
];

/// Extract fold ranges from a tree-sitter syntax tree.
///
/// Traverses the syntax tree to find all foldable nodes and returns their line ranges.
/// The fold range spans from the node's start line to end line (inclusive).
pub fn extract_fold_ranges(tree: &Tree) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    let foldable_types: HashSet<&str> = FOLDABLE_NODE_TYPES.iter().copied().collect();

    let root_node = tree.root_node();
    collect_foldable_nodes(root_node, &foldable_types, &mut ranges);

    // Sort by start line and deduplicate
    ranges.sort_by_key(|r| r.start_line);
    ranges.dedup_by_key(|r| r.start_line);

    ranges
}

/// Extract fold ranges only within a byte range (for incremental updates after edits).
///
/// Skips subtrees entirely outside the range, making it O(nodes in range)
/// instead of O(all nodes in tree).
pub fn extract_fold_ranges_in_range(tree: &Tree, byte_range: Range<usize>) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    let foldable_types: HashSet<&str> = FOLDABLE_NODE_TYPES.iter().copied().collect();

    let root_node = tree.root_node();
    collect_foldable_nodes_in_range(root_node, &foldable_types, &byte_range, &mut ranges);

    ranges.sort_by_key(|r| r.start_line);
    ranges.dedup_by_key(|r| r.start_line);
    ranges
}

/// Recursively collect foldable nodes, skipping subtrees outside byte_range.
fn collect_foldable_nodes_in_range(
    node: Node,
    foldable_types: &HashSet<&str>,
    byte_range: &Range<usize>,
    ranges: &mut Vec<FoldRange>,
) {
    // Skip subtrees entirely outside the target range
    if node.end_byte() <= byte_range.start || node.start_byte() >= byte_range.end {
        return;
    }

    if foldable_types.contains(node.kind()) {
        let start_pos = node.start_position();
        let end_pos = node.end_position();

        if end_pos.row > start_pos.row && (end_pos.row - start_pos.row) >= 2 {
            ranges.push(FoldRange {
                start_line: start_pos.row,
                end_line: end_pos.row,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_foldable_nodes_in_range(child, foldable_types, byte_range, ranges);
    }
}

/// Recursively collect foldable nodes from the syntax tree (full traversal).
fn collect_foldable_nodes(
    node: Node,
    foldable_types: &HashSet<&str>,
    ranges: &mut Vec<FoldRange>,
) {
    if foldable_types.contains(node.kind()) {
        let start_pos = node.start_position();
        let end_pos = node.end_position();

        if end_pos.row > start_pos.row && (end_pos.row - start_pos.row) >= 2 {
            ranges.push(FoldRange {
                start_line: start_pos.row,
                end_line: end_pos.row,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_foldable_nodes(child, foldable_types, ranges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_range_ordering() {
        let mut ranges = vec![
            FoldRange { start_line: 10, end_line: 20 },
            FoldRange { start_line: 5, end_line: 15 },
            FoldRange { start_line: 5, end_line: 15 },
            FoldRange { start_line: 1, end_line: 30 },
        ];

        ranges.sort_by_key(|r| r.start_line);
        ranges.dedup_by_key(|r| r.start_line);

        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0].start_line, 1);
        assert_eq!(ranges[1].start_line, 5);
        assert_eq!(ranges[2].start_line, 10);
    }
}
