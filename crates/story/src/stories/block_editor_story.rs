//! Block editor spike (`block-editor` feature): paragraphs, headings, lists,
//! bold/italic/code marks, cross-block selection, IME, undo/redo.

use gpui_kit::base::text::block_editor::{BlockEditor, BlockEditorEvent, BlockEditorState};
use gpui_kit::component::{button::Button, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled, Subscription, Window, div, px,
};

use crate::section;

const SAMPLE: &str = "# Block editor\n\nType here. **Bold** with Ctrl/Cmd-B, *italic* with Ctrl/Cmd-I, `code` with Ctrl/Cmd-E.\n\n## Lists\n\n- Bullet one\n- Bullet two\n  - Nested (Tab / Shift-Tab)\n\n1. First\n2. Second\n\nUndo with Ctrl/Cmd-Z, redo with Ctrl/Cmd-Shift-Z. Drag or Shift-arrows to select across blocks.";

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
    status: SharedString,
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
        let editor = cx.new(|cx| BlockEditorState::new(window, cx).with_source(SAMPLE));
        let subscription = cx.subscribe(&editor, |this, editor, event: &BlockEditorEvent, cx| {
            if let BlockEditorEvent::Changed { revision, .. } = event {
                let kind = editor.read(cx).block_kind_at_caret();
                this.status = format!("revision {revision}, caret in {kind:?}").into();
                cx.notify();
            }
        });
        Self {
            editor,
            status: "".into(),
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for BlockEditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source = self.editor.read(cx).source();
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("sample").label("Sample").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.editor.update(cx, |e, cx| {
                                let _ = e.set_source(SAMPLE, cx);
                            });
                        },
                    )))
                    .child(
                        Button::new("long")
                            .label("Load 1200 blocks")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.editor.update(cx, |e, cx| {
                                    let _ = e.set_source(&long_document(1200), cx);
                                });
                            })),
                    )
                    .child(div().text_sm().child(self.status.clone())),
            )
            .child(
                section("Editor")
                    .w(px(640.))
                    .child(BlockEditor::new(&self.editor)),
            )
            .child(
                section("Markdown").w(px(640.)).child(
                    div()
                        .text_xs()
                        .font_family("monospace")
                        .whitespace_normal()
                        .child(SharedString::from(
                            source.chars().take(2000).collect::<String>(),
                        )),
                ),
            )
    }
}
