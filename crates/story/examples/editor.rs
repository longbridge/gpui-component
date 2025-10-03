use std::{
    ops::Range,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Ok;
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::{Diagnostic, DiagnosticSeverity, Language, LanguageConfig, LanguageRegistry},
    input::{
        self, CodeActionProvider, CompletionProvider, DefinitionProvider, HoverProvider,
        InputEvent, InputState, Position, Rope, RopeExt, TabSize, TextInput,
    },
    v_flex, ActiveTheme, ContextModal, IconName, IndexPath, Selectable, Sizable,
};
use lsp_types::{
    CodeAction, CodeActionKind, CompletionContext, CompletionItem, CompletionResponse,
    CompletionTextEdit, InsertReplaceEdit, TextEdit, WorkspaceEdit,
};
use story::Assets;

fn init() {
    LanguageRegistry::singleton().register(
        "navi",
        &LanguageConfig::new(
            "navi",
            tree_sitter_navi::LANGUAGE.into(),
            vec![],
            tree_sitter_navi::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
    );
}

pub struct Example {
    editor: Entity<InputState>,
    go_to_line_state: Entity<InputState>,
    language_state: Entity<DropdownState<Vec<SharedString>>>,
    language: Lang,
    line_number: bool,
    need_update: bool,
    soft_wrap: bool,
    lsp_store: ExampleLspStore,
    _subscriptions: Vec<Subscription>,
    _lint_task: Task<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Lang {
    BuiltIn(Language),
    External(&'static str),
}

impl Lang {
    fn name(&self) -> &str {
        match self {
            Lang::BuiltIn(lang) => lang.name(),
            Lang::External(lang) => lang,
        }
    }
}

const LANGUAGES: [(Lang, &'static str); 12] = [
    (
        Lang::BuiltIn(Language::Rust),
        include_str!("./fixtures/test.rs"),
    ),
    (
        Lang::BuiltIn(Language::Markdown),
        include_str!("./fixtures/test.md"),
    ),
    (
        Lang::BuiltIn(Language::Html),
        include_str!("./fixtures/test.html"),
    ),
    (
        Lang::BuiltIn(Language::JavaScript),
        include_str!("./fixtures/test.js"),
    ),
    (
        Lang::BuiltIn(Language::TypeScript),
        include_str!("./fixtures/test.ts"),
    ),
    (
        Lang::BuiltIn(Language::Go),
        include_str!("./fixtures/test.go"),
    ),
    (
        Lang::BuiltIn(Language::Python),
        include_str!("./fixtures/test.py"),
    ),
    (
        Lang::BuiltIn(Language::Ruby),
        include_str!("./fixtures/test.rb"),
    ),
    (
        Lang::BuiltIn(Language::Zig),
        include_str!("./fixtures/test.zig"),
    ),
    (
        Lang::BuiltIn(Language::Sql),
        include_str!("./fixtures/test.sql"),
    ),
    (
        Lang::BuiltIn(Language::Json),
        include_str!("./fixtures/test.json"),
    ),
    (Lang::External("navi"), include_str!("./fixtures/test.nv")),
];

#[derive(Clone)]
pub struct ExampleLspStore {
    completions: Arc<Vec<CompletionItem>>,
    code_actions: Arc<RwLock<Vec<(Range<usize>, CodeAction)>>>,
    diagnostics: Arc<RwLock<Vec<Diagnostic>>>,
    dirty: Arc<RwLock<bool>>,
}

impl ExampleLspStore {
    pub fn new() -> Self {
        let completions = serde_json::from_slice::<Vec<CompletionItem>>(include_bytes!(
            "./fixtures/completion_items.json"
        ))
        .unwrap();

        Self {
            completions: Arc::new(completions),
            code_actions: Arc::new(RwLock::new(vec![])),
            diagnostics: Arc::new(RwLock::new(vec![])),
            dirty: Arc::new(RwLock::new(false)),
        }
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        let guard = self.diagnostics.read().unwrap();
        guard.clone()
    }

    fn update_diagnostics(&self, diagnostics: Vec<Diagnostic>) {
        let mut guard = self.diagnostics.write().unwrap();
        *guard = diagnostics;
        *self.dirty.write().unwrap() = true;
    }

    fn code_actions(&self) -> Vec<(Range<usize>, CodeAction)> {
        let guard = self.code_actions.read().unwrap();
        guard.clone()
    }

    fn update_code_actions(&self, code_actions: Vec<(Range<usize>, CodeAction)>) {
        let mut guard = self.code_actions.write().unwrap();
        *guard = code_actions;
        *self.dirty.write().unwrap() = true;
    }

    fn is_dirty(&self) -> bool {
        let guard = self.dirty.read().unwrap();
        *guard
    }
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

impl CompletionProvider for ExampleLspStore {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let trigger_character = trigger.trigger_character.unwrap_or_default();
        if trigger_character.is_empty() {
            return Task::ready(Ok(CompletionResponse::Array(vec![])));
        }

        // Simulate to delay for fetching completions
        let rope = rope.clone();
        let items = self.completions.clone();
        cx.background_spawn(async move {
            // Simulate a slow completion source, to test Editor async handling.
            smol::Timer::after(Duration::from_millis(20)).await;

            if trigger_character.starts_with("/") {
                let start = offset.saturating_sub(trigger_character.len());
                let start_pos = rope.offset_to_position(start);
                let end_pos = rope.offset_to_position(offset);
                let replace_range = lsp_types::Range::new(start_pos, end_pos);

                let items = vec![
                    completion_item(
                        &replace_range,
                        "/date",
                        format!("{}", chrono::Local::now().date_naive()).as_str(),
                        "Insert current date",
                    ),
                    completion_item(&replace_range, "/thanks", "Thank you!", "Insert Thank you!"),
                    completion_item(&replace_range, "/+1", "👍", "Insert 👍"),
                    completion_item(&replace_range, "/-1", "👎", "Insert 👎"),
                    completion_item(&replace_range, "/smile", "😊", "Insert 😊"),
                    completion_item(&replace_range, "/sad", "😢", "Insert 😢"),
                    completion_item(&replace_range, "/launch", "🚀", "Insert 🚀"),
                ];
                return Ok(CompletionResponse::Array(items));
            }

            let items = items
                .iter()
                .filter(|item| item.label.starts_with(&trigger_character))
                .take(10)
                .map(|item| {
                    let mut item = item.clone();
                    item.insert_text = Some(item.label.replace(&trigger_character, ""));
                    item
                })
                .collect::<Vec<_>>();

            Ok(CompletionResponse::Array(items))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        _new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        true
    }
}

impl CodeActionProvider for ExampleLspStore {
    fn id(&self) -> SharedString {
        "LspStore".into()
    }

    fn code_actions(
        &self,
        _state: Entity<InputState>,
        range: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let mut actions = vec![];
        for (node_range, code_action) in self.code_actions().iter() {
            if !(range.start >= node_range.start && range.end <= node_range.end) {
                continue;
            }

            actions.push(code_action.clone());
        }

        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let Some(edit) = action.edit else {
            return Task::ready(Ok(()));
        };

        let changes = if let Some(changes) = edit.changes {
            changes
        } else {
            return Task::ready(Ok(()));
        };

        let Some((_, text_edits)) = changes.into_iter().next() else {
            return Task::ready(Ok(()));
        };

        let state = state.downgrade();
        window.spawn(cx, async move |cx| {
            state.update_in(cx, |state, window, cx| {
                state.apply_lsp_edits(&text_edits, window, cx);
            })
        })
    }
}

impl HoverProvider for ExampleLspStore {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>> {
        let word = text.word_at(offset);
        if word.is_empty() {
            return Task::ready(Ok(None));
        }

        let Some(item) = self.completions.iter().find(|item| item.label == word) else {
            return Task::ready(Ok(None));
        };

        let contents = if let Some(doc) = &item.documentation {
            match doc {
                lsp_types::Documentation::String(s) => s.clone(),
                lsp_types::Documentation::MarkupContent(mc) => mc.value.clone(),
            }
        } else {
            "No documentation available.".to_string()
        };

        let hover = lsp_types::Hover {
            contents: lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(contents)),
            range: None,
        };

        Task::ready(Ok(Some(hover)))
    }
}

const RUST_DOC_URLS: &[(&str, &str)] = &[
    ("String", "string/struct.String"),
    ("Debug", "fmt/trait.Debug"),
    ("Clone", "clone/trait.Clone"),
    ("Option", "option/enum.Option"),
    ("Result", "result/enum.Result"),
    ("Vec", "vec/struct.Vec"),
    ("HashMap", "collections/hash_map/struct.HashMap"),
    ("HashSet", "collections/hash_set/struct.HashSet"),
    ("Arc", "sync/struct.Arc"),
    ("RwLock", "sync/struct.RwLock"),
    ("Duration", "time/struct.Duration"),
];

impl DefinitionProvider for ExampleLspStore {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>> {
        let Some(word_range) = text.word_range(offset) else {
            return Task::ready(Ok(vec![]));
        };
        let word = text.slice(word_range.clone()).to_string();

        let document_uri = lsp_types::Uri::from_str("file://example").unwrap();
        let start = text.offset_to_position(word_range.start);
        let end = text.offset_to_position(word_range.end);
        let symbol_range = lsp_types::Range { start, end };

        if word == "Duration" {
            let target_range = lsp_types::Range {
                start: lsp_types::Position {
                    line: 2,
                    character: 4,
                },
                end: lsp_types::Position {
                    line: 2,
                    character: 23,
                },
            };
            return Task::ready(Ok(vec![lsp_types::LocationLink {
                target_uri: document_uri,
                target_range: target_range,
                target_selection_range: target_range,
                origin_selection_range: Some(symbol_range),
            }]));
        }

        let names = RUST_DOC_URLS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        for (ix, t) in names.iter().enumerate() {
            if *t == word {
                let url = RUST_DOC_URLS[ix].1;
                let location = lsp_types::LocationLink {
                    target_uri: lsp_types::Uri::from_str(&format!(
                        "https://doc.rust-lang.org/std/{}.html",
                        url
                    ))
                    .unwrap(),
                    target_selection_range: lsp_types::Range::default(),
                    target_range: lsp_types::Range::default(),
                    origin_selection_range: Some(symbol_range),
                };

                return Task::ready(Ok(vec![location]));
            }
        }

        Task::ready(Ok(vec![]))
    }
}

struct TextConvertor;

impl CodeActionProvider for TextConvertor {
    fn id(&self) -> SharedString {
        "TextConvertor".into()
    }

    fn code_actions(
        &self,
        state: Entity<InputState>,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let mut actions = vec![];
        if range.is_empty() {
            return Task::ready(Ok(actions));
        }

        let state = state.read(cx);
        let document_uri = lsp_types::Uri::from_str("file://example").unwrap();

        let old_text = state.text().slice(range.clone()).to_string();
        let start = state.text().offset_to_position(range.start);
        let end = state.text().offset_to_position(range.end);
        let range = lsp_types::Range { start, end };

        actions.push(CodeAction {
            title: "Convert to Uppercase".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text.to_uppercase(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Convert to Lowercase".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range: range.clone(),
                            new_text: old_text.to_lowercase(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Titleize".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range: range.clone(),
                            new_text: old_text
                                .split_whitespace()
                                .map(|word| {
                                    let mut chars = word.chars();
                                    chars
                                        .next()
                                        .map(|c| c.to_uppercase().collect::<String>())
                                        .unwrap_or_default()
                                        + chars.as_str()
                                })
                                .collect::<Vec<_>>()
                                .join(" "),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Capitalize".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .chars()
                                .enumerate()
                                .map(|(i, c)| {
                                    if i == 0 {
                                        c.to_uppercase().to_string()
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .collect(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        // snake_case
        actions.push(CodeAction {
            title: "Convert to snake_case".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .chars()
                                .enumerate()
                                .map(|(i, c)| {
                                    if c.is_uppercase() {
                                        if i != 0 {
                                            format!("_{}", c.to_lowercase())
                                        } else {
                                            c.to_lowercase().to_string()
                                        }
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .collect(),
                            ..Default::default()
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<InputState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let Some(edit) = action.edit else {
            return Task::ready(Ok(()));
        };

        let changes = if let Some(changes) = edit.changes {
            changes
        } else {
            return Task::ready(Ok(()));
        };

        let Some((_, text_edits)) = changes.into_iter().next() else {
            return Task::ready(Ok(()));
        };

        let state = state.downgrade();
        window.spawn(cx, async move |cx| {
            state.update_in(cx, |state, window, cx| {
                state.apply_lsp_edits(&text_edits, window, cx);
            })
        })
    }
}

impl Example {
    pub fn new(default: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_language = if let Some(name) = default {
            LANGUAGES
                .iter()
                .find(|s| s.0.name().starts_with(name.trim()))
                .cloned()
                .unwrap_or(LANGUAGES[0].clone())
        } else {
            LANGUAGES[0].clone()
        };

        let lsp_store = ExampleLspStore::new();

        let editor = cx.new(|cx| {
            let mut editor = InputState::new(window, cx)
                .code_editor(default_language.0.name().to_string())
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value(default_language.1)
                .placeholder("Enter your code here...");

            let lsp_store = Rc::new(lsp_store.clone());
            editor.lsp.completion_provider = Some(lsp_store.clone());
            editor.lsp.code_action_providers = vec![lsp_store.clone(), Rc::new(TextConvertor)];
            editor.lsp.hover_provider = Some(lsp_store.clone());
            editor.lsp.definition_provider = Some(lsp_store.clone());

            editor
        });
        let go_to_line_state = cx.new(|cx| InputState::new(window, cx));
        let language_state = cx.new(|cx| {
            DropdownState::new(
                LANGUAGES.iter().map(|s| s.0.name().into()).collect(),
                Some(IndexPath::default()),
                window,
                cx,
            )
        });

        let _subscriptions = vec![
            cx.subscribe(&editor, |this, _, _: &InputEvent, cx| {
                this.lint_document(cx);
            }),
            cx.subscribe(
                &language_state,
                |this, state, _: &DropdownEvent<Vec<SharedString>>, cx| {
                    if let Some(val) = state.read(cx).selected_value() {
                        if val == "navi" {
                            this.language = Lang::External("navi");
                        } else {
                            this.language = Lang::BuiltIn(Language::from_str(&val));
                        }

                        this.need_update = true;
                        cx.notify();
                    }
                },
            ),
        ];

        Self {
            editor,
            go_to_line_state,
            language_state,
            language: default_language.0,
            line_number: true,
            need_update: false,
            soft_wrap: false,
            lsp_store,
            _subscriptions,
            _lint_task: Task::ready(()),
        }
    }

    fn update_highlighter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.need_update {
            return;
        }

        let language = self.language.name().to_string();
        let code = LANGUAGES.iter().find(|s| s.0.name() == language).unwrap().1;
        self.editor.update(cx, |state, cx| {
            state.set_value(code, window, cx);
            state.set_highlighter(language, cx);
        });

        self.need_update = false;
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

    fn lint_document(&mut self, cx: &mut Context<Self>) {
        let language = self.language.name().to_string();
        let lsp_store = self.lsp_store.clone();
        let text = self.editor.read(cx).text().clone();

        self._lint_task = cx.background_spawn(async move {
            let value = text.to_string();
            let result = autocorrect::lint_for(value.as_str(), &language);

            let mut code_actions = vec![];
            let mut diagnostics = vec![];

            for item in result.lines.iter() {
                let severity = match item.severity {
                    autocorrect::Severity::Error => DiagnosticSeverity::Warning,
                    autocorrect::Severity::Warning => DiagnosticSeverity::Hint,
                    autocorrect::Severity::Pass => DiagnosticSeverity::Info,
                };

                let line = item.line.saturating_sub(1); // Convert to 0-based index
                let col = item.col.saturating_sub(1); // Convert to 0-based index

                let start = Position::new(line as u32, col as u32);
                let end = Position::new(line as u32, (col + item.old.chars().count()) as u32);
                let message = format!("AutoCorrect: {}", item.new);
                diagnostics.push(Diagnostic::new(start..end, message).with_severity(severity));

                let range = text.position_to_offset(&start)..text.position_to_offset(&end);

                let text_edit = TextEdit {
                    range: lsp_types::Range { start, end },
                    new_text: item.new.clone(),
                    ..Default::default()
                };

                let edit = WorkspaceEdit {
                    changes: Some(
                        std::iter::once((
                            lsp_types::Uri::from_str("file://example").unwrap(),
                            vec![text_edit],
                        ))
                        .collect(),
                    ),
                    ..Default::default()
                };

                code_actions.push((
                    range,
                    CodeAction {
                        title: format!("Change to '{}'", item.new),
                        kind: Some(CodeActionKind::QUICKFIX),
                        edit: Some(edit),
                        ..Default::default()
                    },
                ));
            }

            lsp_store.update_code_actions(code_actions.clone());
            lsp_store.update_diagnostics(diagnostics.clone());
        });
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_highlighter(window, cx);

        // Update diagnostics
        if self.lsp_store.is_dirty() {
            let diagnostics = self.lsp_store.diagnostics();
            self.editor.update(cx, |state, cx| {
                state.diagnostics_mut().map(|set| {
                    set.clear();
                    set.extend(diagnostics);
                });
                cx.notify();
            });
        }

        v_flex().size_full().child(
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
                                    Dropdown::new(&self.language_state)
                                        .menu_width(px(160.))
                                        .xsmall(),
                                )
                                .child(
                                    Button::new("line-number")
                                        .ghost()
                                        .when(self.line_number, |this| this.icon(IconName::Check))
                                        .label("Line Number")
                                        .xsmall()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.line_number = !this.line_number;
                                            this.editor.update(cx, |state, cx| {
                                                state.set_line_number(this.line_number, window, cx);
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

    // Parse `cargo run -- <story_name>`
    let name = std::env::args().nth(1);

    app.run(move |cx| {
        story::init(cx);
        init();
        cx.activate(true);

        story::create_new_window_with_size(
            "Editor",
            Some(size(px(1200.), px(960.))),
            |window, cx| cx.new(|cx| Example::new(name, window, cx)),
            cx,
        );
    });
}
