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
        assert!(frame.leaves[0].origin.x > frame.leaves[1].origin.x);
        assert!(frame.leaves[1].line_height > frame.leaves[0].line_height);
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
            l0.origin + gpui::point(px(1.), l0.line_height / 2.),
            l1.origin + gpui::point(px(1000.), l1.line_height / 2.),
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
    let before = editor.read_with(cx, |e, _| (e.projection.built, e.projection.rebuilds));
    draw(cx);
    draw(cx);
    let steady = editor.read_with(cx, |e, _| (e.projection.built, e.projection.rebuilds));
    assert_eq!(before, steady, "steady frames do not re-project");
    caret_at(&editor, cx, 500, 0);
    let t = std::time::Instant::now();
    type_text(cx, "x");
    let elapsed = t.elapsed();
    let after = editor.read_with(cx, |e, _| (e.projection.built, e.projection.rebuilds));
    assert_eq!(after.0 - steady.0, 1, "one leaf re-projected");
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
