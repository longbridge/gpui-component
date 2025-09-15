use std::{rc::Rc, time::Duration};

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::{Diagnostic, DiagnosticSeverity, Language, LanguageConfig, LanguageRegistry},
    input::{self, CompletionProvider, InputEvent, InputState, TabSize, TextInput},
    v_flex, ActiveTheme, ContextModal, IconName, IndexPath, Selectable, Sizable,
};
use lsp_types::{CompletionContext, CompletionItem, CompletionResponse};
use story::Assets;

fn init(cx: &mut App) {
    LanguageRegistry::global_mut(cx).register(
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
    _subscribes: Vec<Subscription>,
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

const COMPLETION_ITEMS: &[&str] = &[
    "as",
    "break",
    "const",
    "continue",
    "crate",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "abstract",
    "alignof",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "offsetof",
    "override",
    "priv",
    "proc",
    "pure",
    "sizeof",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "dyn",
    "async",
    "await",
    "try",
    "union",
    "default",
    "macro_rules",
    "global_allocator",
    "this_is_a_very_long_keyword_to_test_completion",
    "test",
    "bench",
    "cfg",
    "derive",
    "doc",
    "feature",
    "inline",
    "link",
    "macro_use",
    "no_mangle",
    "non_exhaustive",
    "panic_handler",
    "repr",
    "should_panic",
    "target_feature",
    "test_case",
    "thread_local",
    "allow",
    "deny",
    "forbid",
    "warn",
    "cfg_attr",
    "deprecated",
    "must_use",
    "no_std",
    "unstable",
    "alloc",
    "core",
    "std",
    "vec",
    "format",
    "println",
    "eprintln",
    "dbg",
    "todo",
    "unimplemented",
    "unreachable",
    "include",
    "concat",
    "env",
    "option_env",
    "line",
    "column",
    "file",
    "module_path",
    "assert",
    "debug_assert",
];

pub struct ExampleCompletionProvider;

impl CompletionProvider for ExampleCompletionProvider {
    fn completions(
        &self,
        _offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<Vec<CompletionResponse>>> {
        let trigger_character = trigger
            .trigger_character
            .as_deref()
            .unwrap_or("")
            .to_string();
        if trigger_character.is_empty() {
            return Task::ready(Ok(vec![]));
        }

        // Simulate to delay for fetching completions
        cx.background_executor().spawn(async move {
            // Simulate a slow completion source, to test Editor async handling.
            smol::Timer::after(Duration::from_millis(100)).await;

            let items = COMPLETION_ITEMS
                .iter()
                .filter(|s| s.starts_with(&trigger_character))
                .map(|s| CompletionItem::new_simple(s.to_string(), "".to_string()))
                .take(10)
                .collect::<Vec<_>>();

            let responses = vec![CompletionResponse::Array(items)];

            Ok(responses)
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

        let completion_provider = Rc::new(ExampleCompletionProvider);

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

            editor.set_completion_provider(Some(completion_provider), cx);

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

        let _subscribes = vec![
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
            _subscribes,
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
                state.set_placeholder(format!("{}", cursor_pos), window, cx);
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
                        let position =
                            input::Position::new(line.saturating_sub(1), column.saturating_sub(1));

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

    fn lint_document(&self, cx: &mut Context<Self>) {
        // Subscribe to input changes and perform linting with AutoCorrect for markers example.
        let value = self.editor.read(cx).value().clone();
        let result = autocorrect::lint_for(value.as_str(), self.language.name());

        self.editor.update(cx, |state, cx| {
            state.diagnostics_mut().map(|diagnostics| {
                diagnostics.clear();
                for item in result.lines.iter() {
                    let severity = match item.severity {
                        autocorrect::Severity::Error => DiagnosticSeverity::Warning,
                        autocorrect::Severity::Warning => DiagnosticSeverity::Hint,
                        autocorrect::Severity::Pass => DiagnosticSeverity::Info,
                    };

                    let line = item.line.saturating_sub(1); // Convert to 0-based index
                    let col = item.col.saturating_sub(1); // Convert to 0-based index

                    let start = (line, col);
                    let end = (line, col + item.old.chars().count());
                    let message = format!("AutoCorrect: {}", item.new);
                    diagnostics.push(Diagnostic::new(start..end, message).with_severity(severity));
                }
            });

            cx.notify();
        });
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_highlighter(window, cx);

        v_flex().size_full().child(
            v_flex()
                .id("source")
                .w_full()
                .flex_1()
                .child(
                    TextInput::new(&self.editor)
                        .bordered(false)
                        .h_full()
                        .font_family("Monaco")
                        .text_size(px(12.))
                        .focus_bordered(false),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .text_sm()
                        .bg(cx.theme().secondary)
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
                                .label(format!("{} ({} byte)", position, cursor))
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
        init(cx);
        cx.activate(true);

        story::create_new_window_with_size(
            "Code Editor",
            Some(size(px(1200.), px(960.))),
            |window, cx| cx.new(|cx| Example::new(name, window, cx)),
            cx,
        );
    });
}
