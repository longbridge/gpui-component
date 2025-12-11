use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    IconName, Selectable as _, Sizable as _, Size,
    alert::Alert,
    button::{Button, ButtonGroup},
    dock::PanelControl,
    text::markdown,
    v_flex,
};

use crate::section;

pub struct AlertStory {
    size: Size,
    banner_visible: bool,
    focus_handle: gpui::FocusHandle,
}

impl AlertStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            size: Size::default(),
            banner_visible: true,
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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
                section("Default").w_2_3().child(
                    Alert::new(
                        "alert-default",
                        markdown(
                            "This is an alert with icon, title and description (in Markdown).\n\
                            - This is a **list** item.\n\
                            - This is another list item.",
                        ),
                    )
                    .with_size(self.size)
                    .title("Success! Your changes have been saved"),
                ),
            )
            .child(
                section("With variant").w_2_3().child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .child(
                            Alert::info("info1", "This is an info alert.")
                                .with_size(self.size)
                                .title("Info message")
                                .on_close(cx.listener(|_, _, _, _| {
                                    println!("Info alert closed");
                                })),
                        )
                        .child(
                            Alert::success(
                                "success-1",
                                "You have successfully submitted your form.\n\
                        Thank you for your submission!",
                            )
                            .with_size(self.size)
                            .title("Submit Successful"),
                        )
                        .child(
                            Alert::warning(
                                "warning-1",
                                "This is a warning alert with icon, but no title.\n\
                            This is second line of text to test is the line-height is correct.",
                            )
                            .with_size(self.size),
                        )
                        .child(
                            Alert::error(
                                "error-1",
                                markdown(
                                    "Please verify your billing information and try again.\n\
                            - Check your card details\n\
                            - Ensure sufficient funds\n\
                            - Verify billing address",
                                ),
                            )
                            .with_size(self.size)
                            .title("Unable to process your payment."),
                        ),
                ),
            )
            .child(
                section("Banner").w_2_3().child(
                    v_flex()
                        .w_full()
                        .gap_2()
                        .child(
                            Alert::new(
                                "banner-1",
                                "This is a banner alert, it will take \
                       the full width of the container.",
                            )
                            .banner()
                            .on_close(cx.listener(|this, _, _, cx| {
                                this.banner_visible = !this.banner_visible;
                                cx.notify();
                            }))
                            .visible(self.banner_visible)
                            .with_size(self.size),
                        )
                        .child(
                            Alert::info(
                                "banner-info",
                                "This is a banner alert, it will take the full width of the\
                    container.",
                            )
                            .banner()
                            .with_size(self.size),
                        )
                        .child(
                            Alert::success(
                                "banner-success",
                                "This is a banner alert, it will take the full width of the\
                    container.",
                            )
                            .banner()
                            .with_size(self.size),
                        )
                        .child(
                            Alert::warning(
                                "banner-warning",
                                "This is a banner alert, it will take the full width of the\
                    container.",
                            )
                            .banner()
                            .with_size(self.size),
                        )
                        .child(
                            Alert::error(
                                "banner-error",
                                "This is a banner alert, it will take the full width of the\
                    container.",
                            )
                            .banner()
                            .with_size(self.size),
                        ),
                ),
            )
            .child(
                section("Custom Icon").w_2_3().child(
                    Alert::new(
                        "other-1",
                        "Custom icon with info alert with long \
                    long long long long long long long long \
                    long long long long long long long long long \
                    long long messageeeeeeeee.",
                    )
                    .title("Custom Icon")
                    .with_size(self.size)
                    .icon(IconName::Calendar),
                ),
            )
    }
}
