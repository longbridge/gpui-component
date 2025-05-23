use std::sync::LazyLock;

use gpui::*;
use gpui_component::{
    checkbox::Checkbox,
    dropdown::{Dropdown, DropdownEvent, DropdownState},
    h_flex,
    highlighter::{HighlightTheme, Highlighter},
    input::{InputEvent, InputState, TabSize, TextInput},
    v_flex, ActiveTheme as _,
};
use story::Assets;

static LIGHT_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme::default_light());
static DARK_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme::default_dark());

pub struct Example {
    input_state: Entity<InputState>,
    language_state: Entity<DropdownState<Vec<SharedString>>>,
    language: SharedString,
    is_dark: bool,
    line_number: bool,
    _subscribes: Vec<Subscription>,
}

const EXAMPLE: &str = include_str!("./code-editor.rs");
const LANGUAGES: [&str; 7] = ["rust", "javascript", "html", "css", "go", "python", "ruby"];

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_language: SharedString = LANGUAGES[0].into();
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Some(&default_language), &LIGHT_THEME)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(EXAMPLE)
                .placeholder("Enter your code here...")
        });
        let language_state = cx.new(|cx| {
            DropdownState::new(
                LANGUAGES.iter().map(|s| s.to_string().into()).collect(),
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
                        this.update_highlighter(Some(val.clone()), cx);
                        cx.notify();
                    }
                },
            ),
        ];

        Self {
            input_state,
            language_state,
            language: default_language,
            is_dark: false,
            line_number: true,
            _subscribes,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn update_highlighter(&mut self, new_language: Option<SharedString>, cx: &mut Context<Self>) {
        let is_dark = cx.theme().mode.is_dark();
        let is_language_changed = new_language.is_some();
        if new_language.is_some() {
            self.language = new_language.unwrap();
        }
        let language = self.language.as_ref();
        if self.is_dark != is_dark || is_language_changed {
            self.is_dark = is_dark;
            self.input_state.update(cx, |state, cx| {
                if is_dark {
                    state.set_highlighter(Highlighter::new(Some(language), &DARK_THEME), cx);
                } else {
                    state.set_highlighter(Highlighter::new(Some(language), &LIGHT_THEME), cx);
                }
            });
        }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_highlighter(None, cx);

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_4()
                    .pb_0()
                    .gap_4()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .child(Dropdown::new(&self.language_state).title_prefix("Language: "))
                    .child(
                        Checkbox::new("line-numbger")
                            .checked(self.line_number)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.line_number = *checked;
                                this.input_state.update(cx, |state, cx| {
                                    state.set_line_number(this.line_number, window, cx);
                                });
                                cx.notify();
                            }))
                            .label("Line Number"),
                    ),
            )
            .child(
                div()
                    .id("source")
                    .w_full()
                    .flex_1()
                    .font_family("Monaco")
                    .p_4()
                    .text_size(px(12.))
                    .child(TextInput::new(&self.input_state).h_full()),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Code Editor", Example::view, cx);
    });
}
