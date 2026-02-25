use std::collections::HashSet;
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

    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    pub fn line_count(&self) -> usize {
        self.end_line - self.start_line + 1
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

/// Recursively collect foldable nodes from the syntax tree.
fn collect_foldable_nodes(
    node: Node,
    foldable_types: &HashSet<&str>,
    ranges: &mut Vec<FoldRange>,
) {
    let node_type = node.kind();

    // Check if current node is foldable
    if foldable_types.contains(node_type) {
        let start_pos = node.start_position();
        let end_pos = node.end_position();

        // Only fold if:
        // 1. Spans multiple lines (end_line > start_line)
        // 2. Has at least 2+ lines to fold (end_line - start_line >= 2)
        //    This ensures we don't fold single-line or empty blocks
        if end_pos.row > start_pos.row && (end_pos.row - start_pos.row) >= 2 {
            ranges.push(FoldRange {
                start_line: start_pos.row,
                end_line: end_pos.row,
            });
        }
    }

    // Recursively process child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_foldable_nodes(child, foldable_types, ranges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighter::SyntaxHighlighter;
    use ropey::Rope;

    #[test]
    #[cfg(feature = "tree-sitter-languages")]
    fn test_extract_fold_ranges_rust() {
        let code = r#"fn main() {
    let x = 1;
    if x > 0 {
        println!("positive");
    }
}

struct Point {
    x: i32,
    y: i32,
}
"#;

        let rope = Rope::from_str(code);
        let mut highlighter = SyntaxHighlighter::new("rust");
        highlighter.update(None, &rope);

        // 访问内部的 tree
        // 注意：这需要 SyntaxHighlighter 提供访问 tree 的方法
        // 暂时跳过实际测试，只是示例结构
    }

    #[test]
    fn test_fold_range_ordering() {
        let mut ranges = vec![
            FoldRange { start_line: 10, end_line: 20 },
            FoldRange { start_line: 5, end_line: 15 },
            FoldRange { start_line: 5, end_line: 15 }, // 重复
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
