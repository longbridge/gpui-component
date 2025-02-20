use gpui::*;
use gpui_component::{input::TextInput, text::TextView, ActiveTheme as _};
use story::Assets;

pub struct Example {
    text_input: Entity<TextInput>,
    text_view: Entity<TextView>,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("./html.html");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text_input = cx.new(|cx| {
            TextInput::new(window, cx)
                .multi_line()
                .rows(50)
                .placeholder("Input your HTML here...")
        });
        let text_view = cx.new(|cx| TextView::html(EXAMPLE, cx));

        let _subscribe = cx.subscribe(
            &text_input,
            |this, _, _: &gpui_component::input::InputEvent, cx| {
                let new_text = this.text_input.read(cx).text();
                this.text_view.update(cx, |view, cx| {
                    view.set_text(new_text, cx);
                });
            },
        );

        _ = text_input.update(cx, |input, cx| {
            input.set_text(EXAMPLE, window, cx);
        });

        Self {
            text_input,
            text_view,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.text_view.clone()),
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
