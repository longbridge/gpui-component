use gpui::*;
use gpui_component::{h_flex, input::TextInput, text::TextView, ActiveTheme as _};
use story::Assets;

pub struct Example {
    text_input: Entity<TextInput>,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("./html.html");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text_input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line()
                .appearance(false)
                .h_full()
                .placeholder("Enter your HTML here...")
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

        Self {
            text_input,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h_full()
            .child(
                div()
                    .id("source")
                    .h_full()
                    .w_1_2()
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .child(self.text_input.clone()),
            )
            .child(
                div()
                    .id("preview")
                    .h_full()
                    .w_1_2()
                    .p_5()
                    .overflow_y_scroll()
                    .child(TextView::html("preview", self.text_input.read(cx).text())),
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
