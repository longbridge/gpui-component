use std::rc::Rc;

use gpui::*;
use gpui_component::{
    highlighter::HighlightTheme,
    input::TextInput,
    text::{TextView, TextViewStyle},
    ActiveTheme as _,
};
use story::Assets;

pub struct Example {
    text_input: Entity<TextInput>,
}

const EXAMPLE: &str = include_str!("./markdown.md");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text_input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line()
                .appearance(false)
                .h_full()
                .placeholder("Enter your Markdown here...")
        });

        let _subscribe = cx.subscribe(
            &text_input,
            |_, _, _: &gpui_component::input::InputEvent, cx| {
                cx.notify();
            },
        );

        _ = text_input.update(cx, |input, cx| {
            input.set_text(EXAMPLE, window, cx);
        });

        Self { text_input }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = if cx.theme().mode.is_dark() {
            HighlightTheme::default_dark()
        } else {
            HighlightTheme::default_light()
        };

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
                    .child(self.text_input.clone()),
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
                        TextView::markdown("preview", self.text_input.read(cx).text()).style(
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
