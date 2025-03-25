use gpui::*;
use gpui_component::{alert::Alert, v_flex};
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
            .child(Alert::info("This is an info alert.").title("Info message"))
            .child(
                Alert::success(
                    "You have successfully submitted your form.\n\
                    Thank you for your submission!",
                )
                .title("Submit Successful"),
            )
            .child(
                Alert::warning(
                    "This is a warning alert with icon and title.\n\
                    This is second line of text to test is the line-height is correct.",
                )
                .title("Warning!"),
            )
            .child(
                Alert::error(
                    "There was an error submitting your form.\n\
                    Please try again later, if you still have issues, please contact support.",
                )
                .title("Error!"),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Alert Example", Example::view, cx);
    });
}
