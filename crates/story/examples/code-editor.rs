use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::{Language, LanguageConfig, LanguageRegistry},
    input::{InputEvent, InputState, Marker, TabSize, TextInput},
    v_flex, ActiveTheme, Selectable, Sizable,
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
    input_state: Entity<InputState>,
    language_state: Entity<DropdownState<Vec<SharedString>>>,
    language: Lang,
    line_number: bool,
    need_update: bool,
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

const LANGUAGES: [(Lang, &'static str); 10] = [
    (
        Lang::BuiltIn(Language::Rust),
        include_str!("./fixtures/test.rs"),
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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_language = LANGUAGES[0].clone();
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(default_language.0.name().to_string())
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(default_language.1)
                .placeholder("Enter your code here...")
        });
        let language_state = cx.new(|cx| {
            DropdownState::new(
                LANGUAGES.iter().map(|s| s.0.name().into()).collect(),
                Some(0),
                window,
                cx,
            )
        });

        let _subscribes = vec![
            cx.subscribe(&input_state, |_, _, _: &InputEvent, cx| {
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
            input_state,
            language_state,
            language: default_language.0,
            line_number: true,
            need_update: false,
            _subscribes,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn set_markers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.language.name() != "rust" {
            return;
        }

        self.input_state.update(cx, |state, cx| {
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
        self.input_state.update(cx, |state, cx| {
            state.set_value(code, window, cx);
            state.set_highlighter(language, cx);
        });

        self.need_update = false;
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
                .p_4()
                .gap_2()
                .child(
                    TextInput::new(&self.input_state)
                        .h_full()
                        .font_family("Monaco")
                        .text_size(px(12.))
                        .focus_bordered(false),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            h_flex()
                                .gap_3()
                                .child(
                                    Dropdown::new(&self.language_state)
                                        .menu_width(px(160.))
                                        .small(),
                                )
                                .child(
                                    Button::new("line-number")
                                        .ghost()
                                        .label("Line Number")
                                        .small()
                                        .selected(self.line_number)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.line_number = !this.line_number;
                                            this.input_state.update(cx, |state, cx| {
                                                state.set_line_number(this.line_number, window, cx);
                                            });
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child({
                            let loc = self.input_state.read(cx).line_column();
                            let cursor = self.input_state.read(cx).cursor();
                            format!("{} ({} c)", loc, cursor.offset())
                        }),
                ),
        )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        init(cx);
        cx.activate(true);

        story::create_new_window("Code Editor", Example::view, cx);
    });
}
