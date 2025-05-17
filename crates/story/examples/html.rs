use std::sync::LazyLock;

use gpui::*;
use gpui_component::{
    h_flex,
    highlighter::{HighlightTheme, Highlighter},
    input::{InputState, TextInput},
    text::TextView,
    ActiveTheme as _,
};
use story::Assets;

static LIGHT_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme::default_light());
static DARK_THEME: LazyLock<HighlightTheme> = LazyLock::new(|| HighlightTheme::default_dark());

pub struct Example {
    input_state: Entity<InputState>,
    is_dark: bool,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("./html.html");
const LANG: &str = "html";

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line()
                .default_value(EXAMPLE)
                .placeholder("Enter your HTML here...")
                .highlighter(Highlighter::new(Some(LANG), &LIGHT_THEME))
        });

        let _subscribe = cx.subscribe(
            &input_state,
            |_, _, _: &gpui_component::input::InputEvent, cx| {
                cx.notify();
            },
        );

        Self {
            input_state,
            is_dark: false,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();
        if self.is_dark != is_dark {
            self.is_dark = is_dark;
            self.input_state.update(cx, |state, cx| {
                if is_dark {
                    state.set_highlighter(Highlighter::new(Some(LANG), &DARK_THEME), window, cx);
                } else {
                    state.set_highlighter(Highlighter::new(Some(LANG), &LIGHT_THEME), window, cx);
                }
            });
        }

        h_flex()
            .h_full()
            .child(
                div()
                    .id("source")
                    .h_full()
                    .w_1_2()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(TextInput::new(&self.input_state).h_full().appearance(false)),
            )
            .child(
                div()
                    .id("preview")
                    .h_full()
                    .w_1_2()
                    .p_5()
                    .overflow_y_scroll()
                    .child(TextView::html("preview", self.input_state.read(cx).value())),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("HTML Example", Example::view, cx);
    });
}
