use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::{Language, LanguageConfig, LanguageRegistry},
    input::{InputEvent, InputState, Marker, TabSize, TextInput},
    v_flex, ActiveTheme, ContextModal, IconName, IndexPath, Selectable, Sizable,
};
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

        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(default_language.0.name().to_string())
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value(default_language.1)
                .placeholder("Enter your code here...")
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
            cx.subscribe(&editor, |_, _, _: &InputEvent, cx| {
                cx.notify();
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

    fn set_markers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.language.name() != "rust" {
            return;
        }

        self.editor.update(cx, |state, cx| {
            state.set_markers(
                vec![
                    Marker::new("warning", (2, 1), (2, 31), "Import but not used."),
                    Marker::new("error", (16, 10), (16, 46), "Syntax error."),
                    Marker::new("info", (25, 10), (25, 20), "This is a info message, this is a very long message, with **Markdown** support."),
                    Marker::new("hint", (36, 9), (40, 10), "This is a hint message."),
                ],
                window,
                cx,
            );
        });
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
                state.set_placeholder(format!("{}", editor.read(cx).line_column()), window, cx);
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
                        let column = parts.next().and_then(|c| c);

                        editor.update(cx, |state, cx| {
                            state.go_to_line(line, column, window, cx);
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
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_highlighter(window, cx);
        self.set_markers(window, cx);

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
                            let loc = self.editor.read(cx).line_column();
                            let cursor = self.editor.read(cx).cursor();

                            Button::new("line-column")
                                .ghost()
                                .xsmall()
                                .label(format!("{} ({} c)", loc, cursor.offset()))
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

        story::create_new_window(
            "Code Editor",
            |window, cx| cx.new(|cx| Example::new(name, window, cx)),
            cx,
        );
    });
}
