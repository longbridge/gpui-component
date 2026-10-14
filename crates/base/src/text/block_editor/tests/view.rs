//! UI-slice tests: the entity, key bindings, frame seam, IME and perf
//! counters, driven through a real window in `TestAppContext`.

use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler, InteractiveElement as _, IntoElement,
    Modifiers, MouseButton, ParentElement as _, Render, StatefulInteractiveElement as _,
    Styled as _, TestAppContext, VisualTestContext, Window, div, px,
};

use crate::text::block_editor::{
    BlockEditor, BlockEditorState, BlockKind, BlockPosition, BlockSelection,
};

struct Root(Entity<BlockEditorState>);

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(600.))
            .h(px(4000.))
            .child(BlockEditor::new(&self.0))
    }
}

const M: &str = if cfg!(target_os = "macos") {
    "cmd"
} else {
    "ctrl"
};

fn open<'a>(
    cx: &'a mut TestAppContext,
    source: &str,
) -> (Entity<BlockEditorState>, &'a mut VisualTestContext) {
    cx.update(crate::init);
    let source = source.to_string();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        Root(cx.new(|cx| BlockEditorState::new(window, cx).with_source(&source)))
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, cx| {
        let focus = editor.read(cx).focus_handle.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    (editor, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        let _ = window.draw(cx);
    });
}

fn source(editor: &Entity<BlockEditorState>, cx: &VisualTestContext) -> String {
    editor.read_with(cx, |e, _| e.source())
}

fn caret_at(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    leaf: usize,
    offset: usize,
) {
    editor.update(cx, |e, cx| {
        let id = e.model().document().leaves().nth(leaf).unwrap().id();
        e.set_selection(BlockSelection::caret(BlockPosition::new(id, offset)), cx)
            .unwrap();
    });
}

fn type_text(cx: &mut VisualTestContext, text: &str) {
    cx.simulate_input(text);
    draw(cx);
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(&keys.replace("M-", &format!("{M}-")));
    draw(cx);
}

#[gpui::test]
fn typing_and_enter_build_paragraphs(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "hello");
    keys(cx, "enter");
    type_text(cx, "world");
    assert_eq!(source(&editor, cx), "hello\n\nworld");
    keys(cx, "home backspace");
    assert_eq!(source(&editor, cx), "helloworld");
}

#[gpui::test]
fn mark_shortcuts_toggle_stored_and_range_marks(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab");
    caret_at(&editor, cx, 0, 2);
    keys(cx, "M-b");
    type_text(cx, "c");
    keys(cx, "M-b M-i");
    type_text(cx, "d");
    assert_eq!(source(&editor, cx), "ab**c***d*");
    // Range toggle: select "ab" and make it code.
    keys(cx, "home shift-right shift-right M-e");
    assert_eq!(source(&editor, cx), "`ab`**c***d*");
}

#[gpui::test]
fn arrows_cross_blocks_and_vertical_uses_frame(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "# Title\n\n- one\n- two\n\n1. first");
    caret_at(&editor, cx, 0, 5);
    keys(cx, "end right");
    let (leaf, offset) = editor.read_with(cx, |e, _| {
        let caret = e.model().caret().unwrap();
        let ix = e
            .model()
            .document()
            .leaves()
            .position(|l| l.id() == caret.block)
            .unwrap();
        (ix, caret.offset)
    });
    assert_eq!((leaf, offset), (1, 0));
    keys(cx, "down down");
    let kind = editor.read_with(cx, |e, _| e.model().document().leaves().count());
    assert_eq!(kind, 4);
    let at = editor.read_with(cx, |e, _| {
        let caret = e.model().caret().unwrap();
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == caret.block)
    });
    assert_eq!(at, Some(3), "down twice from `one` lands in `first`");
    keys(cx, "up");
    let at = editor.read_with(cx, |e, _| {
        let caret = e.model().caret().unwrap();
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == caret.block)
    });
    assert_eq!(at, Some(2));
}

#[gpui::test]
fn cross_block_selection_then_typing_joins(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "abc\n\ndef\n\nghi");
    caret_at(&editor, cx, 0, 1);
    keys(cx, "shift-down shift-right");
    let ranges = editor.read_with(cx, |e, _| e.selection_ranges().len());
    assert!(ranges >= 2, "selection spans blocks");
    type_text(cx, "X");
    let text = source(&editor, cx);
    assert!(text.starts_with("aX"), "{text}");
    assert!(text.ends_with("ghi"), "{text}");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abc\n\ndef\n\nghi");
    keys(cx, "M-shift-z");
    assert_eq!(source(&editor, cx), text);
}

#[gpui::test]
fn typing_coalesces_and_undo_redo(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "abc");
    keys(cx, "left");
    type_text(cx, "X");
    assert_eq!(source(&editor, cx), "abXc");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abc");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "");
    keys(cx, "M-shift-z M-shift-z");
    assert_eq!(source(&editor, cx), "abXc");
}

#[gpui::test]
fn ime_composition_is_one_undo_step(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a");
    caret_at(&editor, cx, 0, 1);
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "n", None, window, cx);
            e.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            assert_eq!(e.marked_text_range(window, cx), Some(1..3));
            e.replace_text_in_range(None, "你", window, cx);
            assert_eq!(e.marked_text_range(window, cx), None);
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "a你");
    let utf16 =
        cx.update(|window, cx| editor.update(cx, |e, cx| e.selected_text_range(false, window, cx)));
    assert_eq!(utf16.map(|s| s.range), Some(2..2));
    let bounds = cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.bounds_for_range(0..1, Default::default(), window, cx)
        })
    });
    assert!(bounds.is_some());
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "a");
}

#[gpui::test]
fn list_and_heading_commands(cx: &mut TestAppContext) {
    use crate::text::block_editor::EditorCommand;
    let (editor, cx) = open(cx, "one\n\ntwo");
    editor.update(cx, |e, cx| {
        e.run(EditorCommand::ToggleList { ordered: false }, cx)
            .unwrap();
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "- one\n\ntwo");
    caret_at(&editor, cx, 1, 0);
    editor.update(cx, |e, cx| {
        e.run(
            EditorCommand::SetBlockKind(BlockKind::Heading { level: 2 }),
            cx,
        )
        .unwrap();
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "- one\n\n## two");
    // The list leaf is indented in the frame; the heading is not.
    editor.read_with(cx, |e, _| {
        let frame = e.frame.borrow();
        assert!(frame.leaves[0].bounds().left() > frame.leaves[1].bounds().left());
        assert!(frame.leaves[1].line_height() > frame.leaves[0].line_height());
    });
}

#[gpui::test]
fn mouse_click_and_drag_select(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "first block\n\nsecond block");
    let (a, b) = editor.read_with(cx, |e, _| {
        let f = e.frame.borrow();
        let l0 = &f.leaves[0];
        let l1 = &f.leaves[1];
        (
            l0.bounds().origin + gpui::point(px(1.), l0.line_height() / 2.),
            l1.bounds().origin + gpui::point(px(1000.), l1.line_height() / 2.),
        )
    });
    cx.simulate_mouse_down(a, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(b, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(b, MouseButton::Left, Modifiers::none());
    draw(cx);
    type_text(cx, "Z");
    assert_eq!(source(&editor, cx), "Z");
}

/// Perf (D9): steady frames rebuild nothing; a keystroke re-projects one
/// leaf, even in a long document.
#[gpui::test]
fn long_document_reprojects_one_leaf_per_keystroke(cx: &mut TestAppContext) {
    let mut doc = String::new();
    for i in 0..1200 {
        match i % 4 {
            0 => doc.push_str(&format!("## Heading {i}\n\n")),
            1 => doc.push_str(&format!("- item {i} with **bold** and `code`\n\n")),
            2 => doc.push_str(&format!("1. ordered {i}\n\n")),
            _ => doc.push_str(&format!(
                "Paragraph {i} with *italic* text that wraps a little.\n\n"
            )),
        }
    }
    let (editor, cx) = open(cx, &doc);
    let leaves = editor.read_with(cx, |e, _| e.model().document().leaves().count());
    assert!(leaves >= 1000, "{leaves}");
    let before = editor.read_with(cx, |e, _| (e.projection.built(), e.projection.rebuilds));
    draw(cx);
    let frame = std::time::Instant::now();
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let frame = frame.elapsed();
    let steady = editor.read_with(cx, |e, _| (e.projection.built(), e.projection.rebuilds));
    assert_eq!(before, steady, "steady frames do not re-project");
    caret_at(&editor, cx, 500, 0);
    let t = std::time::Instant::now();
    type_text(cx, "x");
    let elapsed = t.elapsed();
    let after = editor.read_with(cx, |e, _| (e.projection.built(), e.projection.rebuilds));
    assert_eq!(after.0 - steady.0, 1, "one node re-projected");
    assert_eq!(after.1 - steady.1, 1);
    eprintln!(
        "perf: {leaves} leaves, full frame {frame:?}, keystroke+frame {elapsed:?} ({} build, test platform)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

fn events(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
) -> std::rc::Rc<std::cell::RefCell<usize>> {
    let count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let c = count.clone();
    cx.update(|_, cx| {
        cx.subscribe(
            editor,
            move |_, event: &crate::text::block_editor::BlockEditorEvent, _| {
                if matches!(
                    event,
                    crate::text::block_editor::BlockEditorEvent::Changed { .. }
                ) {
                    *c.borrow_mut() += 1;
                }
            },
        )
        .detach();
    });
    count
}

/// Review P1: an empty composition must not leave an undo group open.
#[gpui::test]
fn empty_ime_mark_does_not_leak_undo_group(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a");
    caret_at(&editor, cx, 0, 1);
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "", None, window, cx);
            assert!(!e.composition_open());
        })
    });
    type_text(cx, "b");
    keys(cx, "enter");
    type_text(cx, "c");
    assert_eq!(source(&editor, cx), "ab\n\nc");
    keys(cx, "M-z");
    assert_ne!(
        source(&editor, cx),
        "ab\n\nc",
        "undo works after an empty mark"
    );
}

#[gpui::test]
fn cancelled_ime_restores_text_and_closes_group(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab");
    caret_at(&editor, cx, 0, 2);
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "n", None, window, cx);
            e.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            e.replace_and_mark_text_in_range(None, "", None, window, cx);
            assert!(!e.composition_open());
            assert_eq!(e.marked_text_range(window, cx), None);
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "ab");
    type_text(cx, "c");
    assert_eq!(source(&editor, cx), "abc");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "ab");
}

#[gpui::test]
fn ime_over_cross_block_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "abc\n\ndef");
    editor.update(cx, |e, cx| {
        let ids: Vec<_> = e.model().document().leaves().map(|l| l.id()).collect();
        e.set_selection(
            BlockSelection::Text {
                anchor: BlockPosition::new(ids[0], 1),
                head: BlockPosition::new(ids[1], 2),
            },
            cx,
        )
        .unwrap();
    });
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "x", None, window, cx);
            assert_eq!(e.marked_text_range(window, cx), Some(1..2));
            e.replace_text_in_range(None, "X", window, cx);
            assert!(!e.composition_open());
        })
    });
    draw(cx);
    assert_eq!(source(&editor, cx), "aXf");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abc\n\ndef");
}

fn caret_text_and_kind(
    editor: &Entity<BlockEditorState>,
    cx: &VisualTestContext,
) -> (String, Option<BlockKind>) {
    editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        let leaf = e.model().document().block(c.block).unwrap();
        (
            leaf.text().unwrap_or("").to_string(),
            e.block_kind_at_caret(),
        )
    })
}

#[gpui::test]
fn input_rules_make_headings_and_lists(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "## ");
    type_text(cx, "Title");
    keys(cx, "enter");
    type_text(cx, "- ");
    type_text(cx, "item");
    assert_eq!(source(&editor, cx), "## Title\n\n- item");
}

#[gpui::test]
fn ordered_list_input_rule(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "1. ");
    type_text(cx, "first");
    assert_eq!(source(&editor, cx), "1. first");
}

#[gpui::test]
fn undo_reverts_input_rule_but_keeps_typed_marker(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "# ");
    assert_eq!(
        caret_text_and_kind(&editor, cx),
        (String::new(), Some(BlockKind::Heading { level: 1 }))
    );
    keys(cx, "M-z");
    assert_eq!(
        caret_text_and_kind(&editor, cx),
        ("# ".to_string(), Some(BlockKind::Paragraph))
    );
}

#[gpui::test]
fn no_change_event_on_noop_command(cx: &mut TestAppContext) {
    use crate::text::block_editor::EditorCommand;
    let (editor, cx) = open(cx, "plain");
    let count = events(&editor, cx);
    editor.update(cx, |e, cx| {
        let _ = e.run(EditorCommand::Outdent, cx);
    });
    assert_eq!(*count.borrow(), 0);
    editor.update(cx, |e, cx| {
        e.run(EditorCommand::ToggleList { ordered: false }, cx)
            .unwrap();
    });
    assert_eq!(*count.borrow(), 1);
}

#[gpui::test]
fn arrows_collapse_whole_block_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "one\n\ntwo\n\nthree");
    editor.update(cx, |e, cx| {
        let ids: Vec<_> = e
            .model()
            .document()
            .root()
            .children()
            .map(|b| b.id())
            .collect();
        e.set_selection(
            BlockSelection::Blocks {
                anchor: ids[0],
                head: ids[1],
            },
            cx,
        )
        .unwrap();
    });
    keys(cx, "right");
    let caret = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        let ix = e
            .model()
            .document()
            .leaves()
            .position(|l| l.id() == c.block);
        (ix, c.offset)
    });
    assert_eq!(caret, (Some(1), 3));
}

#[gpui::test]
fn backspace_and_delete_join_blocks(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab\n\ncd\n\nef");
    caret_at(&editor, cx, 1, 0);
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "abcd\n\nef");
    keys(cx, "end delete");
    assert_eq!(source(&editor, cx), "abcdef");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "abcd\n\nef");
}

#[gpui::test]
fn paste_multiline_splits_blocks(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab");
    caret_at(&editor, cx, 0, 1);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("X\nY".into()));
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "aX\n\nYb");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "ab");
}

#[gpui::test]
fn readonly_blocks_edits_but_not_navigation(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab\n\ncd");
    editor.update(cx, |e, cx| e.set_readonly(true, cx));
    caret_at(&editor, cx, 0, 2);
    type_text(cx, "x");
    keys(cx, "backspace enter M-b");
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "n", None, window, cx)
        })
    });
    assert_eq!(source(&editor, cx), "ab\n\ncd");
    keys(cx, "right");
    let ix = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == c.block)
    });
    assert_eq!(ix, Some(1));
}

const TABLE_DOC: &str = "a\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n\nb";

/// Window point of `offset` in the `ix`-th document leaf (cells included).
fn leaf_point(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    ix: usize,
    offset: usize,
) -> gpui::Point<gpui::Pixels> {
    draw(cx);
    editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(ix).unwrap().id();
        let b = e
            .frame
            .borrow()
            .caret_bounds(BlockPosition::new(id, offset), px(1.))
            .expect("leaf laid out");
        gpui::point(b.origin.x + px(1.), b.center().y)
    })
}

#[gpui::test]
fn table_cells_click_type_tab_and_cell_selection(cx: &mut TestAppContext) {
    // Leaves: a, x, y, 1, 2, b.
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    // Click in cell "1": a caret, not a node selection.
    let at = leaf_point(&editor, cx, 3, 1);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    assert_eq!(caret_leaf(&editor, cx), Some(3));
    type_text(cx, "0");
    keys(cx, "M-b");
    type_text(cx, "B");
    assert!(
        source(&editor, cx).contains("| 10**B** | 2 |"),
        "{}",
        source(&editor, cx)
    );
    // Arrows cross cell boundaries: right into "2", up into "y", down back.
    keys(cx, "right");
    assert_eq!(caret_leaf(&editor, cx), Some(4));
    keys(cx, "up");
    assert_eq!(caret_leaf(&editor, cx), Some(2));
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(4));
    // Shift+Tab / Tab move between cells; Tab in the last cell adds a row.
    keys(cx, "shift-tab");
    assert_eq!(caret_leaf(&editor, cx), Some(3));
    keys(cx, "tab tab");
    let s = source(&editor, cx);
    assert!(s.contains("| 10**B** | 2 |\n|  |  |"), "{s}");
    type_text(cx, "n");
    assert!(source(&editor, cx).contains("| n |  |"));
    keys(cx, "M-z M-z M-z M-z M-z");
    assert_eq!(source(&editor, cx), original);
    // Drag from "x" to "2": a cell selection; copy gives a table.
    let from = leaf_point(&editor, cx, 1, 0);
    let to = leaf_point(&editor, cx, 4, 1);
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    assert!(editor.read_with(cx, |e, _| matches!(
        e.model().selection(),
        BlockSelection::Cells { .. }
    )));
    assert_eq!(editor.update(cx, |e, _| e.selection_ranges().len()), 4);
    keys(cx, "M-c");
    let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
    assert_eq!(copied, "| x | y |\n| --- | --- |\n| 1 | 2 |");
    // Delete clears contents and keeps the table.
    keys(cx, "backspace");
    assert_eq!(
        source(&editor, cx),
        "a\n\n|  |  |\n| --- | --- |\n|  |  |\n\nb"
    );
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), original);
}

#[gpui::test]
fn backspace_after_table_selects_it_and_copy_round_trips(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 0);
    keys(cx, "backspace");
    assert_eq!(
        source(&editor, cx),
        original,
        "first Backspace only selects"
    );
    keys(cx, "M-c");
    let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
    assert!(copied.starts_with("| x | y |"), "{copied:?}");
    assert!(original.contains(&copied));
    keys(cx, "M-x");
    assert_eq!(source(&editor, cx), "a\n\nb");
}

/// Per-node cache (§7.3): an edit inside a 1000-item list rebuilds only the
/// path from the edited paragraph to the list (paragraph, item, list).
#[gpui::test]
fn edit_inside_big_list_rebuilds_only_the_item_path(cx: &mut TestAppContext) {
    let doc: String = (0..1000).map(|i| format!("- item {i}\n")).collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    let frame = std::time::Instant::now();
    cx.update(|window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    });
    let frame = frame.elapsed();
    let steady = editor.read_with(cx, |e, _| e.projection.built());
    caret_at(&editor, cx, 500, 0);
    let t = std::time::Instant::now();
    type_text(cx, "x");
    let elapsed = t.elapsed();
    let after = editor.read_with(cx, |e, _| e.projection.built());
    assert_eq!(after - steady, 3, "paragraph + item + list");
    assert!(source(&editor, cx).contains("- xitem 500\n"));
    eprintln!(
        "perf: 1000-item list, full frame {frame:?}, keystroke+frame {elapsed:?} ({} build, test platform)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

#[gpui::test]
fn table_rows_are_cached_per_row(cx: &mut TestAppContext) {
    let (editor, cx) = open(
        cx,
        "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n\nafter",
    );
    draw(cx);
    let steady = editor.read_with(cx, |e, _| e.projection.built());
    // Leaves: a, b, 1, 2, 3, 4, after. Typing into cell "3" rebuilds the
    // table and that one row.
    caret_at(&editor, cx, 4, 0);
    type_text(cx, "x");
    assert!(
        source(&editor, cx).contains("| x3 | 4 |"),
        "{}",
        source(&editor, cx)
    );
    let after = editor.read_with(cx, |e, _| e.projection.built());
    assert_eq!(after - steady, 2, "table + one row; other rows reused");
}

fn caret_leaf(editor: &Entity<BlockEditorState>, cx: &VisualTestContext) -> Option<usize> {
    editor.read_with(cx, |e, _| {
        let c = e.model().caret()?;
        e.model()
            .document()
            .leaves()
            .position(|l| l.id() == c.block)
    })
}

/// Culling (light virtualization): only blocks near the viewport are laid
/// out, and up/down still walks every block, including culled ones.
#[gpui::test]
fn culled_blocks_are_navigable(cx: &mut TestAppContext) {
    let doc: String = (0..600).map(|i| format!("paragraph {i}\n\n")).collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    draw(cx);
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 200, "laid out {rendered} of 600");
    caret_at(&editor, cx, 0, 0);
    for _ in 0..450 {
        keys(cx, "down");
    }
    assert_eq!(caret_leaf(&editor, cx), Some(450));
    draw(cx);
    // The caret's block is laid out and has geometry.
    assert!(editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.frame.borrow().leaf(c.block).is_some()
    }));
    for _ in 0..450 {
        keys(cx, "up");
    }
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    // Typing far down edits the right block.
    caret_at(&editor, cx, 599, 0);
    type_text(cx, "x");
    assert!(source(&editor, cx).ends_with("xparagraph 599"));
}

/// A long top-level list is culled in chunks; up/down crosses culled items
/// and ordered numbering continues across chunks.
#[gpui::test]
fn long_list_is_culled_in_chunks_and_navigable(cx: &mut TestAppContext) {
    let doc: String = (0..1000)
        .map(|i| format!("{}. item {i}\n", i + 1))
        .collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    draw(cx);
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 10, "laid out {rendered} chunks of ~32");
    caret_at(&editor, cx, 0, 0);
    for _ in 0..300 {
        keys(cx, "down");
    }
    assert_eq!(caret_leaf(&editor, cx), Some(300));
    type_text(cx, "x");
    assert!(source(&editor, cx).contains("301. xitem 300\n"));
}

struct Scrolled(Entity<BlockEditorState>, gpui::ScrollHandle);

impl Render for Scrolled {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("scroll")
            .w(px(600.))
            .h(px(300.))
            .overflow_y_scroll()
            .track_scroll(&self.1)
            .child(BlockEditor::new(&self.0))
    }
}

/// With a scroll handle, moving the caret into a culled block scrolls it
/// into view once the block is laid out.
#[gpui::test]
fn caret_is_scrolled_into_view(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let doc: String = (0..400).map(|i| format!("paragraph {i}\n\n")).collect();
    let handle = gpui::ScrollHandle::new();
    let scroll = handle.clone();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx).with_source(&doc);
            state.set_scroll_handle(scroll.clone());
            state
        });
        Scrolled(editor, scroll)
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, cx| {
        let focus = editor.read(cx).focus_handle.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    draw(cx);
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 100, "laid out {rendered} of 400");
    caret_at(&editor, cx, 0, 0);
    for _ in 0..200 {
        keys(cx, "down");
    }
    for _ in 0..4 {
        draw(cx);
    }
    assert_eq!(caret_leaf(&editor, cx), Some(200));
    let (caret, view) = editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        (e.frame.borrow().caret_bounds(c, px(1.)), handle.bounds())
    });
    let caret = caret.expect("caret block laid out");
    assert!(
        caret.top() >= view.top() - px(1.) && caret.bottom() <= view.bottom() + px(1.),
        "caret {caret:?} outside view {view:?}"
    );
    assert!(handle.offset().y < px(0.));
}

/// Phase 5 element-state reuse: a block culled while scrolled away keeps
/// its projected node and its shaped text, so scrolling back reshapes
/// nothing and rebuilds no node.
#[gpui::test]
fn scrolled_back_blocks_reuse_nodes_and_layouts(cx: &mut TestAppContext) {
    cx.update(crate::init);
    let doc: String = (0..400).map(|i| format!("paragraph {i}\n\n")).collect();
    let handle = gpui::ScrollHandle::new();
    let scroll = handle.clone();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx).with_source(&doc);
            state.set_scroll_handle(scroll.clone());
            state
        });
        Scrolled(editor, scroll)
    });
    let editor = root.read_with(cx, |root, _| root.0.clone());
    for _ in 0..3 {
        draw(cx);
    }
    let built = editor.read_with(cx, |e, _| e.projection.built());
    // Scroll far down (the top blocks get culled), then back to the top.
    handle.set_offset(gpui::point(px(0.), px(-6000.)));
    for _ in 0..3 {
        draw(cx);
    }
    let culled_top = editor.read_with(cx, |e, _| {
        // Block 1 (block 0 holds the caret and stays pinned).
        let second = e.model().document().leaves().nth(1).unwrap().id();
        e.frame.borrow().leaf(second).is_none()
    });
    assert!(culled_top, "the top blocks are culled while scrolled away");
    handle.set_offset(gpui::point(px(0.), px(0.)));
    crate::text::inline::LAYOUT_REUSE.with(|c| c.set((0, 0)));
    draw(cx);
    let (reused, shaped) = crate::text::inline::LAYOUT_REUSE.with(|c| c.get());
    assert!(reused > 0, "scrolled-back blocks were laid out");
    assert_eq!(shaped, 0, "no block was shaped again ({reused} reused)");
    assert_eq!(
        editor.read_with(cx, |e, _| e.projection.built()),
        built,
        "no node rebuilt"
    );
}

#[gpui::test]
fn copied_table_pastes_back_as_a_table(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 0);
    keys(cx, "backspace");
    keys(cx, "M-x");
    assert_eq!(source(&editor, cx), "a\n\nb");
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), original, "cut + paste round-trips");
    // Pasting again at the end of "b" appends a second copy.
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 1);
    keys(cx, "M-v");
    assert!(
        source(&editor, cx).ends_with("\n\nb\n\n| x | y |\n| --- | --- |\n| 1 | 2 |"),
        "{}",
        source(&editor, cx)
    );
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), original, "paste is one undo step");
}

fn paste(cx: &mut VisualTestContext, text: &str, plain: bool) {
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.into()));
    keys(cx, if plain { "M-shift-v" } else { "M-v" });
}

#[gpui::test]
fn paste_converts_multi_block_markdown(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "xy");
    caret_at(&editor, cx, 0, 1);
    paste(cx, "**p1**\n\n# H\n\n- a\n- b\n\np2", false);
    assert_eq!(source(&editor, cx), "x**p1**\n\n# H\n\n- a\n- b\n\np2y");
    type_text(cx, "Z");
    assert_eq!(source(&editor, cx), "x**p1**\n\n# H\n\n- a\n- b\n\np2Zy");
    keys(cx, "M-z");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "xy", "paste is one undo step");
    // Structure only (no paragraph at either end).
    paste(cx, "# H\n\n- a", false);
    assert_eq!(source(&editor, cx), "x\n\n# H\n\n- a\n\ny");
}

#[gpui::test]
fn single_line_and_plain_paste_stay_inline(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "xy");
    caret_at(&editor, cx, 0, 1);
    paste(cx, "# not a heading", false);
    assert_eq!(source(&editor, cx), "x# not a headingy");
    keys(cx, "M-z");
    paste(cx, "# H\n\n- a", true);
    let src = source(&editor, cx);
    assert!(
        !src.contains("\n# H"),
        "plain paste keeps markers as text: {src:?}"
    );
    assert_eq!(src, "x# H\n\n\\- ay", "markers stay text");
}

/// Spacers are sized from top-to-next-top distances, so a block far down
/// sits at the same y whether the blocks above it are culled or not.
#[gpui::test]
fn culled_spacers_keep_block_positions(cx: &mut TestAppContext) {
    let doc: String = (0..300)
        .map(|i| match i % 3 {
            0 => format!("## Heading {i}\n\n"),
            1 => format!("- item {i}\n\n---\n\n"),
            _ => format!("paragraph {i}\n\n"),
        })
        .collect();
    let (editor, cx) = open(cx, &doc);
    // A text leaf well down the document (rules are atoms with no layout).
    let (target, id) = editor.read_with(cx, |e, _| {
        let doc = &e.model().doc;
        doc.leaves()
            .iter()
            .enumerate()
            .skip(250)
            .find(|(_, id)| doc.inline(**id).is_some())
            .map(|(ix, id)| (ix, *id))
            .unwrap()
    });
    caret_at(&editor, cx, target, 0);
    draw(cx);
    // A frame with culling off (no visible span yet) lays out everything.
    editor.update(cx, |e, cx| {
        e.cull.borrow_mut().visible = None;
        cx.notify();
    });
    cx.update(|window, cx| {
        let _ = window.draw(cx);
    });
    let full = editor.read_with(cx, |e, _| {
        e.frame.borrow().leaf(id).map(|l| l.bounds().top())
    });
    let full = full.expect("unculled frame");
    draw(cx);
    draw(cx);
    let (rendered, culled) = editor.read_with(cx, |e, _| {
        (
            e.cull.borrow().rendered,
            e.frame.borrow().leaf(id).map(|l| l.bounds().top()),
        )
    });
    assert!(rendered < 300, "culling active ({rendered})");
    let culled = culled.expect("caret block pinned");
    assert!((culled - full).abs() < px(0.5), "{culled:?} vs {full:?}");
}

#[gpui::test]
fn end_at_soft_wrap_stays_on_the_visual_line(cx: &mut TestAppContext) {
    let long = "word ".repeat(60);
    let (editor, cx) = open(cx, long.trim_end());
    caret_at(&editor, cx, 0, 0);
    draw(cx);
    keys(cx, "end");
    let (line_top, caret_top, offset) = editor.read_with(cx, |e, _| {
        let f = e.frame.borrow();
        let c = e.model().caret().unwrap();
        let leaf = f.leaf(c.block).unwrap();
        (
            leaf.caret_point(0).y,
            leaf.caret_point(c.offset).y,
            c.offset,
        )
    });
    assert!(offset > 0 && offset < long.trim_end().len(), "{offset}");
    assert_eq!(caret_top, line_top, "End stays on the first visual line");
    keys(cx, "home");
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        0
    );
}

/// Without a scroll handle, caret-into-view is a no-op (no panic, flag
/// cleared).
#[gpui::test]
fn reveal_without_scroll_handle_is_a_noop(cx: &mut TestAppContext) {
    let doc: String = (0..400).map(|i| format!("p {i}\n\n")).collect();
    let (editor, cx) = open(cx, &doc);
    caret_at(&editor, cx, 399, 0);
    draw(cx);
    draw(cx);
    assert!(editor.read_with(cx, |e, _| e.scroll_handle.is_none() && !e.reveal.get()));
}

#[gpui::test]
fn code_fence_input_rule_and_code_editing(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "```rust");
    keys(cx, "enter");
    assert_eq!(source(&editor, cx), "```rust\n\n```");
    type_text(cx, "fn a() {");
    keys(cx, "enter tab");
    type_text(cx, "x");
    keys(cx, "enter");
    type_text(cx, "}");
    assert_eq!(source(&editor, cx), "```rust\nfn a() {\n    x\n}\n```");
    // The renderer draws the code (highlighted) and reports its layout.
    draw(cx);
    assert!(editor.read_with(cx, |e, _| {
        let c = e.model().caret().unwrap();
        e.frame.borrow().caret_bounds(c, px(1.)).is_some()
    }));
    // IME inside code.
    cx.update(|window, cx| {
        editor.update(cx, |e, cx| {
            e.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            e.replace_and_mark_text_in_range(None, "你", None, window, cx);
            e.replace_text_in_range(None, "你", window, cx);
        })
    });
    assert_eq!(source(&editor, cx), "```rust\nfn a() {\n    x\n}你\n```");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "```rust\nfn a() {\n    x\n}\n```");
    // Arrow down past the end leaves the block.
    keys(cx, "down");
    type_text(cx, "after");
    assert_eq!(
        source(&editor, cx),
        "```rust\nfn a() {\n    x\n}\n```\n\nafter"
    );
}

#[gpui::test]
fn exiting_code_block_then_typing_is_two_undo_steps(cx: &mut TestAppContext) {
    for exit in ["enter enter", "down"] {
        let (editor, cx) = open(cx, "```\ncode\n```");
        let id = editor.read_with(cx, |e, _| {
            e.model().document().leaves().next().unwrap().id()
        });
        editor.update(cx, |e, _| {
            e.model
                .set_selection(BlockSelection::caret(BlockPosition::new(id, 4)))
                .unwrap()
        });
        keys(cx, exit);
        type_text(cx, "after");
        assert_eq!(source(&editor, cx), "```\ncode\n```\n\nafter", "{exit}");
        keys(cx, "M-z");
        assert!(
            !source(&editor, cx).contains("after"),
            "{exit}: typing undone first"
        );
        assert_eq!(
            editor.read_with(cx, |e, _| e.model().document().leaves().count()),
            2,
            "{exit}"
        );
        keys(cx, "M-z");
        assert_eq!(
            editor.read_with(cx, |e, _| e.model().document().leaves().count()),
            1,
            "{exit}"
        );
    }
}

#[gpui::test]
fn code_block_highlighter_is_applied(cx: &mut TestAppContext) {
    use std::sync::{Arc, Mutex};
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let (editor, cx) = open(cx, "```rust\nfn a\n```");
    let log = seen.clone();
    cx.update(|_, cx| {
        crate::TextViewDefaults::new()
            .with_code_block_highlighter(move |block| {
                log.lock().unwrap().push(block.code().to_string());
                vec![(
                    0..2,
                    gpui::HighlightStyle {
                        color: Some(gpui::red()),
                        ..Default::default()
                    },
                )]
            })
            .install(cx)
    });
    editor.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert!(
        seen.lock().unwrap().iter().any(|c| c == "fn a"),
        "the editor's code block is highlighted: {:?}",
        seen.lock().unwrap()
    );
    // Editing re-highlights the new code.
    let id = editor.read_with(cx, |e, _| {
        e.model().document().leaves().next().unwrap().id()
    });
    editor.update(cx, |e, _| {
        e.model
            .set_selection(BlockSelection::caret(BlockPosition::new(id, 4)))
            .unwrap()
    });
    type_text(cx, "()");
    draw(cx);
    assert!(seen.lock().unwrap().iter().any(|c| c == "fn a()"));
}

#[gpui::test]
fn code_fence_with_space_converts(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "");
    type_text(cx, "``` ");
    type_text(cx, "code");
    assert_eq!(source(&editor, cx), "```\ncode\n```");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "```\n\n```");
    keys(cx, "M-z");
    assert_eq!(
        source(&editor, cx),
        "\\`\\`\\`",
        "undo restores the typed fence"
    );
}

/// Review fix: typing over a node-selected table replaces it with a
/// paragraph holding the text, as one undo step.
#[gpui::test]
fn typing_over_selected_table_replaces_it(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let last = editor.read_with(cx, |e, _| e.model().document().leaves().count() - 1);
    caret_at(&editor, cx, last, 0);
    keys(cx, "backspace");
    type_text(cx, "z");
    assert_eq!(source(&editor, cx), "a\n\nz\n\nb");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), original, "one undo step");
    keys(cx, "M-shift-z");
    assert_eq!(source(&editor, cx), "a\n\nz\n\nb", "redo replays both");
}

/// Phase 4d (#3113 semantics): an inline atom is one editing unit.
#[gpui::test]
fn inline_atom_is_one_unit(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a![pic](x.png)b");
    caret_at(&editor, cx, 0, 1);
    // Shift+Right selects the whole atom; copy gives its Markdown.
    keys(cx, "shift-right");
    let (anchor, head) = editor.read_with(cx, |e, _| match e.model().selection() {
        BlockSelection::Text { anchor, head } => (anchor.offset, head.offset),
        _ => panic!("text selection"),
    });
    assert_eq!((anchor, head), (1, 4));
    keys(cx, "M-c");
    let copied = cx.read_from_clipboard().and_then(|c| c.text()).unwrap();
    assert_eq!(copied, "![pic](x.png)");
    // Right moves over it in one step; Backspace removes it whole; undo restores.
    caret_at(&editor, cx, 0, 1);
    keys(cx, "right");
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        4
    );
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "ab");
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "a![pic](x.png)b");
    // Typing next to it never merges into it.
    caret_at(&editor, cx, 0, 4);
    type_text(cx, "c");
    assert_eq!(source(&editor, cx), "a![pic](x.png)cb");
    // Cut of a selection holding it, then paste, round-trips.
    caret_at(&editor, cx, 0, 0);
    keys(cx, "shift-right shift-right shift-right M-x");
    assert_eq!(source(&editor, cx), "b");
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "a![pic](x.png)cb");
}

/// Paragraphs with inline code render through the single-`Inline` path in
/// the editor, so they report a layout (caret, clicks, up/down).
#[gpui::test]
fn inline_code_paragraph_is_laid_out(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a `b` c");
    draw(cx);
    let at = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f
            .leaf(id)
            .expect("inline-code paragraph reports its layout");
        let p = leaf.caret_point(3);
        gpui::point(p.x + px(0.5), p.y + leaf.line_height() / 2.)
    });
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        3
    );
}

/// Window point at the middle of the inline atom starting at `offset` in the
/// first leaf.
fn atom_point(
    editor: &Entity<BlockEditorState>,
    cx: &mut VisualTestContext,
    offset: usize,
) -> gpui::Point<gpui::Pixels> {
    draw(cx);
    editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f.leaf(id).expect("mixed paragraph reports its pieces");
        let l = leaf.caret_point(offset);
        let r = leaf.caret_point(offset + 3);
        gpui::point((l.x + r.x) / 2., l.y + leaf.line_height() / 2.)
    })
}

#[gpui::test]
fn image_paragraph_has_caret_clicks_and_atom_selection(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab![pic](x.png)cd\n\nnext");
    draw(cx);
    // Laid out: carets before and after the atom are distinct, text after
    // the image is right of it.
    let (before, after, end) = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f.leaf(id).expect("mixed paragraph reports its pieces");
        (
            leaf.caret_point(2),
            leaf.caret_point(5),
            leaf.caret_point(7),
        )
    });
    assert!(
        after.x >= before.x && end.x > after.x,
        "{before:?} {after:?} {end:?}"
    );
    // Click in the text after the image places the caret there.
    let at = gpui::point(end.x - px(1.), end.y + px(2.));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    let caret = editor.read_with(cx, |e, _| e.model().caret().unwrap());
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    assert!(caret.offset >= 6, "{caret:?}");
    // Down goes to the next paragraph, up comes back into the mixed one.
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(1));
    keys(cx, "up");
    assert_eq!(caret_leaf(&editor, cx), Some(0));
    // A click on the image selects it whole.
    assert!(after.x > before.x, "the image has width");
    {
        let at = atom_point(&editor, cx, 2);
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        let sel = editor.read_with(cx, |e, _| e.model().selection().clone());
        let BlockSelection::Text { anchor, head } = sel else {
            panic!("{sel:?}")
        };
        assert_eq!((anchor.offset, head.offset), (2, 5));
        // Selection paints on the text pieces too.
        keys(cx, "shift-right shift-right");
        assert!(!editor.update(cx, |e, _| e.selection_ranges()).is_empty());
    }
}

#[gpui::test]
fn image_only_paragraph_is_laid_out(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "a\n\n![pic](x.png)\n\nb");
    draw(cx);
    let laid_out = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(1).unwrap().id();
        e.frame.borrow().leaf(id).is_some()
    });
    assert!(laid_out);
    caret_at(&editor, cx, 0, 1);
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(1));
    keys(cx, "down");
    assert_eq!(caret_leaf(&editor, cx), Some(2));
}

/// Inline code keeps its pill (InlineFlow) in the editor, and the caret
/// geometry follows the padded text.
#[gpui::test]
fn inline_code_keeps_pill_and_caret_geometry(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "ab `cd` ef");
    draw(cx);
    let (pieces, b, c, d) = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        let f = e.frame.borrow();
        let leaf = f.leaf(id).unwrap();
        (
            leaf.pieces.len(),
            leaf.caret_point(3),
            leaf.caret_point(4),
            leaf.caret_point(5),
        )
    });
    assert!(pieces > 1, "rendered by the flow, in fragments");
    assert!(b.x < c.x && c.x < d.x, "{b:?} {c:?} {d:?}");
    // Clicking between `c` and `d` puts the caret at 4.
    let at = gpui::point(c.x, c.y + px(3.));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().unwrap().offset),
        4
    );
    type_text(cx, "X");
    assert_eq!(source(&editor, cx), "ab `cXd` ef");
}

/// Phase 5: frames without an edit reuse the cached cull units and height
/// index (no per-frame walk of every block); an edit rebuilds them once.
#[gpui::test]
fn cull_units_are_cached_per_revision(cx: &mut TestAppContext) {
    let doc: String = (0..2000).map(|i| format!("paragraph {i}\n\n")).collect();
    let (editor, cx) = open(cx, &doc);
    draw(cx);
    draw(cx);
    let rebuilds =
        |cx: &mut VisualTestContext| editor.read_with(cx, |e, _| e.cull.borrow().index.rebuilds);
    let steady = rebuilds(cx);
    caret_at(&editor, cx, 1, 0);
    keys(cx, "down down right");
    draw(cx);
    assert_eq!(rebuilds(cx), steady, "selection changes do not rebuild");
    type_text(cx, "x");
    draw(cx);
    assert_eq!(rebuilds(cx), steady + 1, "one rebuild per edit");
    let rendered = editor.read_with(cx, |e, _| e.cull.borrow().rendered);
    assert!(rendered < 400, "culled: {rendered} of 2000 laid out");
}

/// Phase 5 accessibility: per-block roles and the D8 root value.
#[gpui::test]
fn accessibility_roles_and_cached_value(cx: &mut TestAppContext) {
    use gpui::Role;
    let (editor, cx) = open(
        cx,
        "# Title\n\ntext ![i](x.png)\n\n- a\n\n```\ncode\n```\n\n---",
    );
    draw(cx);
    let roles: Vec<_> = editor.update(cx, |e, _| {
        e.snapshot()
            .blocks
            .iter()
            .flat_map(|b| b.nodes.iter())
            .map(|n| super::super::element::block_role(n))
            .collect()
    });
    assert_eq!(
        roles,
        vec![
            Some((Role::Heading, Some(1))),
            Some((Role::Paragraph, None)),
            Some((Role::List, None)),
            Some((Role::Code, None)),
            Some((Role::Splitter, None)),
        ]
    );
    let value = editor.read_with(cx, |e, _| e.accessible_value());
    assert_eq!(value.as_ref(), "Title\ntext \na\ncode");
    // Cached for the current revision.
    let cached = editor.read_with(cx, |e, _| {
        e.a11y_value.borrow().as_ref().map(|(rev, _)| *rev) == Some(e.model().revision())
    });
    assert!(cached);
    caret_at(&editor, cx, 0, 5);
    type_text(cx, "!");
    let edited = editor.read_with(cx, |e, _| e.accessible_value());
    assert!(edited.starts_with("Title!\n"));
}

fn leaf_text(editor: &Entity<BlockEditorState>, cx: &VisualTestContext, ix: usize) -> String {
    editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(ix).unwrap().id();
        e.model.doc.leaf_text(id).unwrap_or_default().to_string()
    })
}

fn type_chars(cx: &mut VisualTestContext, text: &str) {
    for ch in text.chars() {
        type_text(cx, &ch.to_string());
    }
}

#[gpui::test]
fn inline_mark_input_rules(cx: &mut TestAppContext) {
    for (typed, expected) in [
        ("a `x`", "a `x`"),
        ("a **b**", "a **b**"),
        ("a *i*", "a *i*"),
        ("a ~~s~~", "a ~~s~~"),
    ] {
        let (editor, cx) = open(cx, "");
        type_chars(cx, typed);
        assert_eq!(source(&editor, cx), expected, "{typed}");
        let marked = &expected[2..];
        let plain = marked.trim_matches(['`', '*', '~']);
        assert_eq!(
            leaf_text(&editor, cx, 0),
            format!("a {plain}"),
            "delimiters dropped"
        );
        // Typing on is outside the span.
        type_text(cx, "z");
        assert_eq!(source(&editor, cx), format!("{expected}z"), "{typed}");
        // Undo: first the "z", then the rule (literal delimiters come back).
        keys(cx, "M-z");
        assert_eq!(source(&editor, cx), expected);
        keys(cx, "M-z");
        assert_eq!(leaf_text(&editor, cx, 0), typed, "undo restores {typed}");
    }
}

#[gpui::test]
fn inline_mark_rules_do_not_fire_in_code_or_on_padding(cx: &mut TestAppContext) {
    // Inside an inline code span: literal.
    let (editor, cx) = open(cx, "`ab`");
    caret_at(&editor, cx, 0, 1);
    type_chars(cx, "*x*");
    assert_eq!(source(&editor, cx), "`a*x*b`");
    // Space-padded and half-bold: no rule.
    let (editor, cx) = open(cx, "");
    type_chars(cx, "2 * 3 *");
    assert_eq!(leaf_text(&editor, cx, 0), "2 * 3 *");
    let (editor, cx) = open(cx, "");
    type_chars(cx, "**a*");
    assert_eq!(leaf_text(&editor, cx, 0), "**a*");
    type_text(cx, "*");
    assert_eq!(source(&editor, cx), "**a**");
}

#[gpui::test]
fn inline_code_shortcut_button_paste_and_marks(cx: &mut TestAppContext) {
    // Cmd/Ctrl-E over a selection.
    let (editor, cx) = open(cx, "a x b");
    editor.update(cx, |e, cx| {
        let id = e.model().document().leaves().next().unwrap().id();
        e.set_selection(
            BlockSelection::Text {
                anchor: BlockPosition::new(id, 2),
                head: BlockPosition::new(id, 3),
            },
            cx,
        )
        .unwrap();
    });
    keys(cx, "M-e");
    assert_eq!(source(&editor, cx), "a `x` b");
    // The story toolbar button runs the same command.
    editor.update(cx, |e, cx| {
        e.run(
            crate::text::block_editor::EditorCommand::ToggleMark(
                crate::text::block_editor::MarkKind::Code,
            ),
            cx,
        )
        .unwrap();
    });
    assert_eq!(source(&editor, cx), "a x b");
    // Bold over code leaves the code alone.
    keys(cx, "M-z");
    assert_eq!(source(&editor, cx), "a `x` b");
    keys(cx, "M-a M-b");
    assert_eq!(source(&editor, cx), "**a** `x` **b**");
    // It renders through the inline-code pill path (flow fragments).
    let pieces = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().next().unwrap().id();
        e.frame.borrow().leaf(id).map(|l| l.pieces.len())
    });
    assert!(pieces.unwrap() > 1);
    // Paste of `x` is code, and round-trips.
    let (editor, cx) = open(cx, "");
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("`x`".into()));
    keys(cx, "M-v");
    assert_eq!(source(&editor, cx), "`x`");
    assert_eq!(leaf_text(&editor, cx, 0), "x");
}

/// QA: empty paragraphs, list items, headings and table cells get a caret,
/// sized to the font (ascent + descent), not the full line box.
#[gpui::test]
fn empty_leaves_have_a_font_sized_caret(cx: &mut TestAppContext) {
    for (src, keys_) in [
        ("a", "end enter"),
        ("- a", "end enter"),
        ("# a", "end backspace"),
        ("| x | y |\n| --- | --- |\n|  | 2 |", ""),
    ] {
        let (editor, cx) = open(cx, src);
        caret_at(&editor, cx, if src.starts_with('|') { 2 } else { 0 }, 0);
        if !keys_.is_empty() {
            keys(cx, keys_);
        }
        draw(cx);
        let (caret, line) = editor.read_with(cx, |e, _| {
            let c = e.model().caret().unwrap();
            let f = e.frame.borrow();
            let leaf = f.leaf(c.block).expect("empty leaf is laid out");
            assert_eq!(e.model().document().leaves().count() > 0, true);
            (f.caret_bounds(c, px(1.)).unwrap(), leaf.bounds())
        });
        assert!(caret.size.height > px(0.), "{src}");
        assert!(
            caret.size.height < line.size.height,
            "{src}: caret {caret:?} line {line:?}"
        );
        assert!(line.contains(&caret.center()), "{src}");
    }
}

/// QA: a click anywhere in an empty cell's area puts the caret in that cell,
/// and Backspace after a table selects the table node, then deletes it.
#[gpui::test]
fn empty_cell_click_and_table_node_backspace(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, "| x | y |\n| --- | --- |\n|  |  |\n\nb");
    let (cell, at) = editor.read_with(cx, |e, _| {
        let id = e.model().document().leaves().nth(3).unwrap().id();
        let b = e.frame.borrow().leaf(id).unwrap().bounds();
        (id, gpui::point(b.left() + px(2.), b.center().y))
    });
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    draw(cx);
    assert_eq!(
        editor.read_with(cx, |e, _| e.model().caret().map(|c| c.block)),
        Some(cell)
    );
    caret_at(&editor, cx, 4, 0);
    keys(cx, "backspace");
    assert!(editor.read_with(cx, |e, _| matches!(
        e.model().selection(),
        BlockSelection::Blocks { .. }
    )));
    assert!(
        source(&editor, cx).contains("| x |"),
        "first Backspace only selects"
    );
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "b");
}

/// QA: the block handle. A click in the left edge of a table or code block
/// (left of its text) selects it as a node; Backspace then deletes it.
#[gpui::test]
fn left_edge_click_node_selects_table_and_code_block(cx: &mut TestAppContext) {
    for src in [
        "a\n\n| x | y |\n| --- | --- |\n| 1 | 2 |\n\nb",
        "a\n\n```\ncode\n```\n\nb",
    ] {
        let (editor, cx) = open(cx, src);
        let at = editor.read_with(cx, |e, _| {
            let id = e.model().document().leaves().nth(1).unwrap().id();
            let b = e.frame.borrow().leaf(id).unwrap().bounds();
            gpui::point(b.left() - px(4.), b.center().y)
        });
        assert!(at.x > px(0.), "{src}: the edge is inside the editor");
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        draw(cx);
        assert!(
            editor.read_with(cx, |e, _| matches!(
                e.model().selection(),
                BlockSelection::Blocks { .. }
            )),
            "{src}"
        );
        keys(cx, "backspace");
        assert_eq!(source(&editor, cx), "a\n\nb", "{src}");
    }
}
