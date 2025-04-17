use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    alert::Alert,
    button::{Button, ButtonGroup},
    dock::PanelControl,
    v_flex, IconName, Selectable as _, Sizable as _, Size,
};

use crate::section;

pub struct AlertStory {
    size: Size,
    focus_handle: gpui::FocusHandle,
}

impl AlertStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            size: Size::default(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl super::Story for AlertStory {
    fn title() -> &'static str {
        "Alert"
    }

    fn description() -> &'static str {
        "Displays a callout for user attention."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for AlertStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AlertStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
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
                section("Info").max_w_md().child(
                    Alert::info("This is an info alert.")
                        .with_size(self.size)
                        .title("Info message"),
                ),
            )
            .child(
                section("Success with Title").max_w_md().child(
                    Alert::success(
                        "You have successfully submitted your form.\n\
                    Thank you for your submission!",
                    )
                    .with_size(self.size)
                    .title("Submit Successful"),
                ),
            )
            .child(
                section("Warning").max_w_md().child(
                    Alert::warning(
                        "This is a warning alert with icon and title.\n\
                    This is second line of text to test is the line-height is correct.",
                    )
                    .with_size(self.size),
                ),
            )
            .child(
                section("Error").max_w_md().child(
                    Alert::error(
                        "There was an error submitting your form.\n\
                    Please try again later, if you still have issues, please contact support.",
                    )
                    .with_size(self.size)
                    .title("Error!"),
                ),
            )
            .child(
                section("Custom Icon").max_w_md().child(
                    Alert::info("Custom icon with info alert.")
                        .title("Custom Icon")
                        .with_size(self.size)
                        .icon(IconName::Bell),
                ),
            )
    }
}
