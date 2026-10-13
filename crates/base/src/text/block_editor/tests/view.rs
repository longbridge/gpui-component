//! UI-slice tests: the entity, key bindings, frame seam, IME and perf
//! counters, driven through a real window in `TestAppContext`.

use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler, IntoElement, Modifiers, MouseButton,
    ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window, div, px,
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
    draw(cx);
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
        "perf: {leaves} leaves, keystroke+frame {elapsed:?} ({} build, test platform)",
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

#[gpui::test]
fn click_selects_table_as_node_then_delete_and_undo(cx: &mut TestAppContext) {
    let (editor, cx) = open(cx, TABLE_DOC);
    let original = source(&editor, cx);
    let (center, cells) = editor.read_with(cx, |e, _| {
        let f = e.frame.borrow();
        let (_, bounds) = f.tables.first().copied().expect("table has bounds");
        (bounds.center(), f.tables.len())
    });
    assert_eq!(cells, 1);
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    draw(cx);
    assert!(editor.read_with(cx, |e, _| matches!(
        e.model().selection(),
        BlockSelection::Blocks { .. }
    )));
    // The selected table's cells get selection backgrounds.
    let painted = editor.update(cx, |e, _| e.selection_ranges().len());
    assert_eq!(painted, 4);
    keys(cx, "backspace");
    assert_eq!(source(&editor, cx), "a\n\nb");
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
    let steady = editor.read_with(cx, |e, _| e.projection.built());
    caret_at(&editor, cx, 500, 0);
    let t = std::time::Instant::now();
    type_text(cx, "x");
    let elapsed = t.elapsed();
    let after = editor.read_with(cx, |e, _| e.projection.built());
    assert_eq!(after - steady, 3, "paragraph + item + list");
    assert!(source(&editor, cx).contains("- xitem 500\n"));
    eprintln!(
        "perf: 1000-item list, keystroke+frame {elapsed:?} ({} build, test platform)",
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
    // Leaves: a, b, 1, 2, 3, 4, after. Typing into cell "3" (model-level;
    // there is no cell editing UI) rebuilds the table and that one row.
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
