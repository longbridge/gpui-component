use gpui::*;
use gpui_component::{
    alert::Alert,
    button::{Button, ButtonGroup},
    v_flex, IconName, Selectable as _, Sizable as _, Size,
};
use story::Assets;

pub struct Example {
    size: Size,
}

impl Example {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            size: Size::default(),
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .m_4()
            .gap_4()
            .child(
                ButtonGroup::new("toggle-size")
                    .outline()
                    .compact()
                    .child(
                        Button::new("xsmall")
                            .label("XSmall")
                            .selected(self.size == Size::XSmall),
                    )
                    .child(
                        Button::new("small")
                            .label("Small")
                            .selected(self.size == Size::Small),
                    )
                    .child(
                        Button::new("medium")
                            .label("Medium")
                            .selected(self.size == Size::Medium),
                    )
                    .child(
                        Button::new("large")
                            .label("Large")
                            .selected(self.size == Size::Large),
                    )
                    .on_click(cx.listener(|this, selecteds: &Vec<usize>, window, cx| {
                        let size = match selecteds[0] {
                            0 => Size::XSmall,
                            1 => Size::Small,
                            2 => Size::Medium,
                            3 => Size::Large,
                            _ => unreachable!(),
                        };
                        this.set_size(size, window, cx);
                    })),
            )
            .child(
                Alert::info("This is an info alert.")
                    .with_size(self.size)
                    .title("Info message"),
            )
            .child(
                Alert::success(
                    "You have successfully submitted your form.\n\
                    Thank you for your submission!",
                )
                .with_size(self.size)
                .title("Submit Successful"),
            )
            .child(
                Alert::warning(
                    "This is a warning alert with icon and title.\n\
                    This is second line of text to test is the line-height is correct.",
                )
                .with_size(self.size),
            )
            .child(
                Alert::error(
                    "There was an error submitting your form.\n\
                    Please try again later, if you still have issues, please contact support.",
                )
                .with_size(self.size)
                .title("Error!"),
            )
            .child(
                Alert::info("Custom icon with info alert.")
                    .with_size(self.size)
                    .icon(IconName::Bell),
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
