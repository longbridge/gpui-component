use std::{
    ops::Range,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
};

use anyhow::Ok;
use gpui::{prelude::FluentBuilder, *};
use gpui_component::input::DefinitionProvider;
use gpui_component::{
    ActiveTheme, ContextModal, IconName, IndexPath, Rope, Selectable, Sizable,
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::{Diagnostic, Language},
    input::{self, HoverProvider, InputEvent, InputState, RopeExt, TabSize, TextInput},
    v_flex,
};
use lsp_types::{CodeAction, CompletionItem, CompletionTextEdit, InsertReplaceEdit};
use story::{Assets, Open};

use crate::{rust_analyzer::RustAnalyzerLspProvider, text_convetor::TextConvertor};

mod document_colors;
mod rust_analyzer;
mod text_convetor;

pub struct Example {
    editor: Entity<InputState>,
    go_to_line_state: Entity<InputState>,
    line_number: bool,
    need_update: bool,
    soft_wrap: bool,
    rust_analyzer: Rc<RustAnalyzerLspProvider>,
    _subscriptions: Vec<Subscription>,
    _lint_task: Task<()>,
}

fn completion_item(
    replace_range: &lsp_types::Range,
    label: &str,
    replace_text: &str,
    documentation: &str,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(lsp_types::CompletionItemKind::FUNCTION),
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: replace_text.to_string(),
            insert: replace_range.clone(),
            replace: replace_range.clone(),
        })),
        documentation: Some(lsp_types::Documentation::String(documentation.to_string())),
        insert_text: None,
        ..Default::default()
    }
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let work_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
        let rust_analyzer = Rc::new(RustAnalyzerLspProvider::new(work_dir).unwrap());

        let editor = cx.new(|cx| {
            let mut editor = InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value("")
                .placeholder("Enter your code here...");

            editor.lsp.completion_provider = Some(rust_analyzer.clone());
            editor.lsp.code_action_providers = vec![rust_analyzer.clone(), Rc::new(TextConvertor)];
            // editor.lsp.hover_provider = Some(rust_analyzer.clone());
            // editor.lsp.definition_provider = Some(rust_analyzer.clone());
            editor.lsp.document_color_provider = Some(rust_analyzer.clone());

            editor
        });
        let go_to_line_state = cx.new(|cx| InputState::new(window, cx));

        let _subscriptions = vec![cx.subscribe(&editor, |this, _, _: &InputEvent, cx| {
            // this.lint_document(cx);
        })];

        Self {
            editor,
            rust_analyzer,
            go_to_line_state,
            line_number: true,
            need_update: false,
            soft_wrap: false,
            _subscriptions,
            _lint_task: Task::ready(()),
        }
    }

    fn go_to_line(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        let input_state = self.go_to_line_state.clone();

        window.open_modal(cx, move |modal, window, cx| {
            input_state.update(cx, |state, cx| {
                let cursor_pos = editor.read(cx).cursor_position();
                state.set_placeholder(
                    format!("{}:{}", cursor_pos.line, cursor_pos.character),
                    window,
                    cx,
                );
                state.focus(window, cx);
            });

            modal
                .title("Go to line")
                .child(TextInput::new(&input_state))
                .confirm()
                .on_ok({
                    let editor = editor.clone();
                    let input_state = input_state.clone();
                    move |_, window, cx| {
                        let query = input_state.read(cx).value();
                        let mut parts = query
                            .split(':')
                            .map(|s| s.trim().parse::<usize>().ok())
                            .collect::<Vec<_>>()
                            .into_iter();
                        let Some(line) = parts.next().and_then(|l| l) else {
                            return false;
                        };
                        let column = parts.next().and_then(|c| c).unwrap_or(1);
                        let position = input::Position::new(
                            line.saturating_sub(1) as u32,
                            column.saturating_sub(1) as u32,
                        );

                        editor.update(cx, |state, cx| {
                            state.set_cursor_position(position, window, cx);
                        });

                        true
                    }
                })
        });
    }

    fn toggle_soft_wrap(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.soft_wrap = !self.soft_wrap;
        self.editor.update(cx, |state, cx| {
            state.set_soft_wrap(self.soft_wrap, window, cx);
        });
        cx.notify();
    }

    fn on_action_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Select a source file".into()),
        });

        let editor = self.editor.clone();
        let rust_analyzer = self.rust_analyzer.clone();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??.iter().next()?.clone();

            rust_analyzer.open_file(path.clone()).ok()?;

            let language = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default();
            let language = Language::from_str(&language);
            let content = std::fs::read_to_string(&path).ok()?;

            window
                .update(|window, cx| {
                    _ = editor.update(cx, |this, cx| {
                        this.set_highlighter(language.name(), cx);
                        this.set_value(content, window, cx);
                    });
                })
                .ok();

            Some(())
        })
        .detach();
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("app")
            .size_full()
            .on_action(cx.listener(Self::on_action_open))
            .child(
                v_flex()
                    .id("source")
                    .w_full()
                    .flex_1()
                    .child(
                        TextInput::new(&self.editor)
                            .bordered(false)
                            .p_0()
                            .h_full()
                            .font_family("Monaco")
                            .text_size(px(12.))
                            .focus_bordered(false),
                    )
                    .child(
                        h_flex()
                            .justify_between()
                            .text_sm()
                            .bg(cx.theme().background)
                            .py_1p5()
                            .px_4()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(
                                        Button::new("line-number")
                                            .ghost()
                                            .when(self.line_number, |this| {
                                                this.icon(IconName::Check)
                                            })
                                            .label("Line Number")
                                            .xsmall()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.line_number = !this.line_number;
                                                this.editor.update(cx, |state, cx| {
                                                    state.set_line_number(
                                                        this.line_number,
                                                        window,
                                                        cx,
                                                    );
                                                });
                                                cx.notify();
                                            })),
                                    )
                                    .child({
                                        Button::new("soft-wrap")
                                            .ghost()
                                            .xsmall()
                                            .label("Soft Wrap")
                                            .selected(self.soft_wrap)
                                            .on_click(cx.listener(Self::toggle_soft_wrap))
                                    }),
                            )
                            .child({
                                let position = self.editor.read(cx).cursor_position();
                                let cursor = self.editor.read(cx).cursor();

                                Button::new("line-column")
                                    .ghost()
                                    .xsmall()
                                    .label(format!(
                                        "{}:{} ({} byte)",
                                        position.line + 1,
                                        position.character + 1,
                                        cursor
                                    ))
                                    .on_click(cx.listener(Self::go_to_line))
                            }),
                    ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window_with_size(
            "Editor",
            Some(size(px(1200.), px(960.))),
            |window, cx| cx.new(|cx| Example::new(window, cx)),
            cx,
        );
    });
}
