use gpui::*;
use gpui_component::{text_view::MarkdownView, ActiveTheme as _};
use story::Assets;

pub struct Example {
    markdown: Entity<MarkdownView>,
}

const EXMAPLE: &str = include_str!("./markdown.md");

impl Example {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let markdown = cx.new(|_| MarkdownView::new(EXMAPLE));

        Self { markdown }
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
                    .border_1()
                    .border_color(cx.theme().border)
                    .flex_1()
                    .p_5()
                    .overflow_y_scroll()
                    .child(EXMAPLE),
            )
            .child(
                div()
                    .id("preview")
                    .h_full()
                    .w_1_2()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_5()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.markdown.clone()),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Modal Example", Example::view, cx);
    });
}
