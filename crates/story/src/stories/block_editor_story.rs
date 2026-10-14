//! Block editor spike (`block-editor` feature): paragraphs, headings, lists,
//! bold/italic/code marks, cross-block selection, IME, undo/redo.

use gpui_kit::base::text::block_editor::{
    BlockEditor, BlockEditorEvent, BlockEditorState, BlockKind, EditorCommand, MarkKind, TableEdit,
};
use gpui_kit::component::{button::Button, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _,
    Styled, Subscription, Window, div, rems,
};

use crate::section;

const SAMPLE: &str = "# Block editor\n\nType here. **Bold** with Ctrl/Cmd-B, *italic* with Ctrl/Cmd-I, `code` with Ctrl/Cmd-E (or type `code`, **bold**, *italic*, ~~strike~~).\n\n## Lists\n\n- Bullet one\n- Bullet two\n  - Nested (Tab / Shift-Tab)\n\n1. First\n2. Second\n\nUndo with Ctrl/Cmd-Z, redo with Ctrl/Cmd-Shift-Z. Drag or Shift-arrows to select across blocks.\n\n```rust\nfn main() {\n    println!(\"code blocks: type ``` then Enter\");\n}\n```\n\n| Feature | Status |\n| --- | --- |\n| Tables | click a cell to edit, Tab to move |\n| Lists | Enter on an empty item exits |";

/// A generated document of 1000+ blocks for the perf note.
pub fn long_document(blocks: usize) -> String {
    let mut doc = String::new();
    for i in 0..blocks {
        match i % 4 {
            0 => doc.push_str(&format!("## Section {i}\n\n")),
            1 => doc.push_str(&format!("- Item {i} with **bold** and `code`\n\n")),
            2 => doc.push_str(&format!("1. Ordered {i}\n\n")),
            _ => doc.push_str(&format!(
                "Paragraph {i} with *italic* text that is long enough to wrap onto a second line in a narrow panel.\n\n"
            )),
        }
    }
    doc
}

pub struct BlockEditorStory {
    editor: Entity<BlockEditorState>,
    /// The editor's scroll container; the editor keeps the caret in view.
    scroll: ScrollHandle,
    status: SharedString,
    /// Markdown preview, refreshed on `Changed` only (never per frame).
    markdown: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for BlockEditorStory {
    fn title() -> &'static str {
        "BlockEditor"
    }

    fn description() -> &'static str {
        "Block-based WYSIWYG Markdown editor (experimental spike)."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl BlockEditorStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let scroll = ScrollHandle::new();
        let editor = cx.new(|cx| {
            let mut state = BlockEditorState::new(window, cx).with_source(SAMPLE);
            state.set_scroll_handle(scroll.clone());
            state
        });
        let subscription = cx.subscribe(&editor, |this, editor, event: &BlockEditorEvent, cx| {
            if let BlockEditorEvent::Changed { revision, .. } = event {
                let kind = editor.read(cx).block_kind_at_caret();
                this.status = format!("revision {revision}, caret in {kind:?}").into();
                this.refresh_markdown(cx);
                cx.notify();
            }
        });
        let mut this = Self {
            editor,
            scroll,
            status: "".into(),
            markdown: "".into(),
            _subscriptions: vec![subscription],
        };
        this.refresh_markdown(cx);
        this
    }

    fn refresh_markdown(&mut self, cx: &App) {
        let source = self.editor.read(cx).source();
        self.markdown = source.chars().take(2000).collect::<String>().into();
    }

    fn load(&mut self, source: &str, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| {
            let _ = e.set_source(source, cx);
        });
        self.refresh_markdown(cx);
        cx.notify();
    }

    fn command_button(
        id: &'static str,
        label: &'static str,
        command: EditorCommand,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                let command = command.clone();
                this.editor.update(cx, |e, cx| {
                    let _ = e.run(command, cx);
                    e.focus(window, cx);
                });
            }))
    }
}

impl Render for BlockEditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(Self::command_button(
                        "bold",
                        "B",
                        EditorCommand::ToggleMark(MarkKind::Bold),
                        cx,
                    ))
                    .child(Self::command_button(
                        "italic",
                        "I",
                        EditorCommand::ToggleMark(MarkKind::Italic),
                        cx,
                    ))
                    .child(Self::command_button(
                        "strike",
                        "S",
                        EditorCommand::ToggleMark(MarkKind::Strikethrough),
                        cx,
                    ))
                    .child(Self::command_button(
                        "inline-code",
                        "`code`",
                        EditorCommand::ToggleMark(MarkKind::Code),
                        cx,
                    ))
                    .child(Self::command_button(
                        "p",
                        "Text",
                        EditorCommand::SetBlockKind(BlockKind::Paragraph),
                        cx,
                    ))
                    .child(Self::command_button(
                        "h1",
                        "H1",
                        EditorCommand::SetBlockKind(BlockKind::Heading { level: 1 }),
                        cx,
                    ))
                    .child(Self::command_button(
                        "h2",
                        "H2",
                        EditorCommand::SetBlockKind(BlockKind::Heading { level: 2 }),
                        cx,
                    ))
                    .child(Self::command_button(
                        "h3",
                        "H3",
                        EditorCommand::SetBlockKind(BlockKind::Heading { level: 3 }),
                        cx,
                    ))
                    .child(Self::command_button(
                        "ul",
                        "Bullets",
                        EditorCommand::ToggleList { ordered: false },
                        cx,
                    ))
                    .child(Self::command_button(
                        "ol",
                        "Numbered",
                        EditorCommand::ToggleList { ordered: true },
                        cx,
                    ))
                    .child(Self::command_button(
                        "code",
                        "Code",
                        EditorCommand::SetCodeBlock { lang: None },
                        cx,
                    ))
                    .child(Self::command_button(
                        "table",
                        "Table",
                        EditorCommand::InsertTable { rows: 3, cols: 3 },
                        cx,
                    ))
                    .child(Self::command_button(
                        "row+",
                        "+Row",
                        EditorCommand::EditTable(TableEdit::AddRowBelow),
                        cx,
                    ))
                    .child(Self::command_button(
                        "row-",
                        "-Row",
                        EditorCommand::EditTable(TableEdit::RemoveRow),
                        cx,
                    ))
                    .child(Self::command_button(
                        "col+",
                        "+Col",
                        EditorCommand::EditTable(TableEdit::AddColumnRight),
                        cx,
                    ))
                    .child(Self::command_button(
                        "col-",
                        "-Col",
                        EditorCommand::EditTable(TableEdit::RemoveColumn),
                        cx,
                    ))
                    .child(
                        Button::new("sample")
                            .label("Sample")
                            .on_click(cx.listener(|this, _, _, cx| this.load(SAMPLE, cx))),
                    )
                    .child(Button::new("long").label("Load 1200 blocks").on_click(
                        cx.listener(|this, _, _, cx| this.load(&long_document(1200), cx)),
                    ))
                    .child(div().text_sm().child(self.status.clone())),
            )
            .child(
                section("Editor").child(
                    div()
                        .id("block-editor-scroll")
                        .flex_1()
                        .w_full()
                        .max_h(rems(28.))
                        .overflow_y_scroll()
                        // Scroll containment: wheel events over this pane
                        // must not also scroll the story/gallery.
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .track_scroll(&self.scroll)
                        .child(BlockEditor::new(&self.editor)),
                ),
            )
            .child(
                section("Markdown").child(
                    div()
                        .id("block-editor-markdown")
                        .flex_1()
                        .w_full()
                        .max_h(rems(28.))
                        .overflow_y_scroll()
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .text_xs()
                        .whitespace_normal()
                        .child(self.markdown.clone()),
                ),
            )
    }
}
