//! Commands on nested blocks and the editing primitives.

use super::*;

#[test]
fn split_and_join_paragraphs() {
    let mut m = model("abcd");
    caret_in(&mut m, 0, 2);
    m.split_block().unwrap();
    assert_eq!(m.source(), "ab\n\ncd");
    m.delete(false).unwrap();
    assert_eq!(m.source(), "abcd");
    assert_eq!(m.caret().unwrap().offset, 2);
}

#[test]
fn split_in_list_item_creates_item() {
    let mut m = model("- [x] ab");
    caret_in(&mut m, 0, 1);
    m.split_block().unwrap();
    assert_eq!(m.source(), "- [x] a\n- [ ] b");
    m.insert_text("z", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "- [x] a\n- [ ] zb");
}

#[test]
fn newline_in_inserted_text_splits() {
    let mut m = model("");
    m.insert_text("a\nb", EditOrigin::Paste).unwrap();
    assert_eq!(m.source(), "a\n\nb");
}

#[test]
fn delete_forward_joins_next() {
    let mut m = model("ab\n\ncd");
    caret_at_end(&mut m, 0);
    m.delete(true).unwrap();
    assert_eq!(m.source(), "abcd");
}

#[test]
fn backspace_removes_preceding_rule() {
    let mut m = model("a\n\n---\n\nb");
    caret_in(&mut m, 2, 0);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "a\n\nb");
}

#[test]
fn backspace_removes_whole_grapheme() {
    let mut m = model("e\u{301}x");
    caret_in(&mut m, 0, 3);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "x");
}

#[test]
fn cross_block_delete_joins_ends() {
    let mut m = model("# abc\n\n- def\n\nghi");
    let leaves: Vec<_> = m.document().leaves().map(|b| b.id()).collect();
    m.set_selection(BlockSelection::Text {
        anchor: BlockPosition::new(leaves[2], 1),
        head: BlockPosition::new(leaves[0], 1),
    })
    .unwrap();
    m.run(EditorCommand::DeleteSelection).unwrap();
    assert_eq!(m.source(), "# ahi");
    assert_invariants(&m);
}

#[test]
fn nested_list_toggle_converts_inner_list_only() {
    let mut m = model("- a\n  - b");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::ToggleList { ordered: true }).unwrap();
    assert_eq!(m.source(), "- a\n  1. b");
    m.run(EditorCommand::ToggleList { ordered: true }).unwrap();
    assert_eq!(m.source(), "- a\n\n  b");
    assert_invariants(&m);
}

#[test]
fn toggle_list_off_splits_list() {
    let mut m = model("- a\n- b\n- c");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::ToggleList { ordered: false }).unwrap();
    assert_eq!(m.source(), "- a\n\nb\n\n- c");
    assert_invariants(&m);
}

#[test]
fn indent_then_outdent() {
    let mut m = model("- a\n- b\n- c");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::Indent).unwrap();
    assert_eq!(m.source(), "- a\n  - b\n- c");
    caret_in(&mut m, 2, 0);
    m.run(EditorCommand::Indent).unwrap();
    assert_eq!(m.source(), "- a\n  - b\n  - c");
    caret_in(&mut m, 1, 0);
    m.run(EditorCommand::Outdent).unwrap();
    assert_eq!(m.source(), "- a\n- b\n  - c");
    assert_invariants(&m);
}

#[test]
fn move_block_inside_blockquote() {
    let mut m = model("> a\n>\n> b");
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::MoveBlocks { down: true }).unwrap();
    assert_eq!(m.source(), "> b\n>\n> a");
}

#[test]
fn set_block_kind_and_query() {
    let mut m = model("a");
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::SetBlockKind(BlockKind::Heading { level: 2 }))
        .unwrap();
    assert_eq!(m.source(), "## a");
    assert_eq!(
        m.block_kind_at_caret(),
        Some(BlockKind::Heading { level: 2 })
    );
}

#[test]
fn toggle_task() {
    let mut m = model("- a");
    caret_in(&mut m, 0, 0);
    m.run(EditorCommand::ToggleTask).unwrap();
    assert_eq!(m.source(), "- [ ] a");
}

#[test]
fn mark_state_and_active_marks() {
    let mut m = model("a **b** c");
    select(&mut m, 0, 0..3);
    assert_eq!(m.mark_state(MarkKind::Bold), MarkState::Mixed);
    caret_in(&mut m, 0, 3);
    assert!(m.active_marks().has(MarkKind::Bold));
    caret_in(&mut m, 0, 1);
    assert!(!m.active_marks().has(MarkKind::Bold));
}

#[test]
fn link_does_not_extend_at_trailing_edge() {
    let mut m = model("[ab](https://x.dev)");
    caret_at_end(&mut m, 0);
    m.insert_text("c", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "[ab](https://x.dev)c");
    caret_in(&mut m, 0, 1);
    m.insert_text("z", EditOrigin::Typing).unwrap();
    assert_eq!(m.source(), "[azb](https://x.dev)c");
}

#[test]
fn typing_into_code_block_is_not_editable_yet() {
    let mut m = model("```\ncode\n```");
    caret_in(&mut m, 0, 0);
    assert!(matches!(
        m.insert_text("x", EditOrigin::Typing),
        Err(CommandError::NotEditable(_))
    ));
    assert_eq!(m.source(), "```\ncode\n```");
}

#[test]
fn invalid_offsets_are_errors() {
    let mut m = model("é");
    let id = m.document().first_caret().block;
    assert_eq!(
        m.set_selection(BlockSelection::caret(BlockPosition::new(id, 1))),
        Err(SelectionError::Position(
            PositionError::NotOnGraphemeBoundary
        ))
    );
    assert_eq!(
        m.set_selection(BlockSelection::caret(BlockPosition::new(id, 9))),
        Err(SelectionError::Position(PositionError::OffsetOutOfBounds {
            len: 2
        }))
    );
    let root = m.document().root().id();
    assert_eq!(
        m.set_selection(BlockSelection::caret(BlockPosition::new(root, 0))),
        Err(SelectionError::Position(PositionError::NotACaretBlock(
            root
        )))
    );
}

#[test]
fn input_order_is_document_order() {
    let m = model("> a\n\n- b\n  - c\n\n| d | e |\n| --- | --- |\n| f | g |");
    let texts: Vec<_> = m.document().leaves().filter_map(|b| b.text()).collect();
    assert_eq!(texts, ["a", "b", "c", "d", "e", "f", "g"]);
}

/// Enter, Enter at the end of `leaf`, then type `x`; one undo reverts the exit.
fn exit_list_after(source: &str, leaf: usize, offset: usize) -> (String, BlockEditorModel) {
    let mut m = model(source);
    caret_in(&mut m, leaf, offset);
    m.split_block().unwrap();
    let before = m.source();
    let revision = m.revision();
    m.split_block().unwrap();
    assert_invariants(&m);
    assert!(m.revision() > revision);
    let after = m.source();
    assert!(m.undo());
    assert_eq!(m.source(), before, "exit is one undo step");
    assert!(m.redo());
    assert_eq!(m.source(), after);
    m.insert_text("x", EditOrigin::Typing).unwrap();
    (m.source(), m)
}

#[test]
fn enter_in_empty_last_item_exits_list() {
    let (source, m) = exit_list_after("- a\n- b", 1, 1);
    assert_eq!(source, "- a\n- b\n\nx");
    assert_invariants(&m);
}

#[test]
fn enter_in_empty_middle_item_splits_list() {
    let (source, m) = exit_list_after("1. a\n2. b", 0, 1);
    assert_eq!(source, "1. a\n\nx\n\n1. b");
    assert_invariants(&m);
}

#[test]
fn enter_in_empty_nested_item_outdents() {
    let (source, m) = exit_list_after("- a\n  - b", 1, 1);
    assert_eq!(source, "- a\n  - b\n- x");
    assert_invariants(&m);
}

#[test]
fn table_node_selection_moves_and_deletes() {
    let mut m = model("a\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n\nb");
    let original = m.source();
    let last = m.document().leaves().count() - 1;
    caret_in(&mut m, last, 0);
    m.delete(false).unwrap();
    assert_eq!(m.source(), original, "Backspace after a table selects it");
    assert!(matches!(m.selection(), BlockSelection::Blocks { .. }));
    assert!(m.run(EditorCommand::MoveBlocks { down: false }).unwrap());
    assert!(m.source().starts_with("| x | y |"), "{}", m.source());
    assert_invariants(&m);
    m.delete(false).unwrap();
    assert_eq!(m.source(), "a\n\nb");
    assert!(m.undo());
    assert!(m.source().starts_with("| x | y |"));
    assert_invariants(&m);
}
