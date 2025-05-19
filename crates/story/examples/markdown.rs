use std::{rc::Rc, sync::LazyLock};

use gpui::*;
use gpui_component::{
    highlighter::{HighlightTheme, Highlighter},
    input::{InputEvent, InputState, TextInput},
    text::{TextView, TextViewStyle},
    ActiveTheme as _,
};
use story::Assets;

static LIGHT_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme::default_light());
static DARK_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme::default_dark());
const LANG: &str = "markdown";

pub struct Example {
    input_state: Entity<InputState>,
    is_dark: bool,
}

const EXAMPLE: &str = include_str!("./markdown.md");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Some(LANG), &LIGHT_THEME)
                .placeholder("Enter your Markdown here...")
                .default_value(EXAMPLE)
        });

        let _subscribe = cx.subscribe(&input_state, |_, _, _: &InputEvent, cx| {
            cx.notify();
        });

        Self {
            input_state,
            is_dark: false,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = if cx.theme().mode.is_dark() {
            HighlightTheme::default_dark()
        } else {
            HighlightTheme::default_light()
        };

        let is_dark = cx.theme().mode.is_dark();
        if self.is_dark != is_dark {
            self.is_dark = is_dark;
            self.input_state.update(cx, |state, cx| {
                if is_dark {
                    state.set_highlighter(Highlighter::new(Some(LANG), &DARK_THEME), cx);
                } else {
                    state.set_highlighter(Highlighter::new(Some(LANG), &LIGHT_THEME), cx);
                }
            });
        }

        div()
            .flex()
            .flex_row()
            .h_full()
            .child(
                div()
                    .id("source")
                    .h_full()
                    .w_1_2()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .flex_1()
                    .child(TextInput::new(&self.input_state).h_full().appearance(false)),
            )
            .child(
                div()
                    .id("preview")
                    .h_full()
                    .w_1_2()
                    .p_5()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        TextView::markdown("preview", self.input_state.read(cx).value()).style(
                            TextViewStyle {
                                highlight_theme: Rc::new(theme),
                                ..Default::default()
                            },
                        ),
                    ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Markdown Example", Example::view, cx);
    });
}
