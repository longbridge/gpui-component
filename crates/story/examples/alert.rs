use gpui::*;
use gpui_component::{alert::Alert, v_flex, IconName};
use story::Assets;

pub struct Example {}

impl Example {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {}
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .m_4()
            .gap_4()
            .child(Alert::new("This is a info alert."))
            .child(
                Alert::new(
                    "This is a warning alert with icon and title. \
                    This is second line of text to test is the line-height is correct.",
                )
                .title("Warning!")
                .icon(IconName::TriangleAlert),
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
