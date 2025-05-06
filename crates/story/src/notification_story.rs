use gpui::{
    div, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement, Render, Styled, Window,
};

use gpui_component::{
    button::{Button, ButtonVariants as _},
    notification::{Notification, NotificationType},
    ContextModal as _,
};

use crate::section;

pub struct NotificationStory {
    focus_handle: FocusHandle,
}

impl super::Story for NotificationStory {
    fn title() -> &'static str {
        "Notification"
    }

    fn description() -> &'static str {
        "Push notifications to display a message at the top right of the window"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl NotificationStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for NotificationStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NotificationStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("notification-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                section("Simple Notification").child(
                    Button::new("show-notify-0")
                        .label("Show Notification")
                        .on_click(cx.listener(|_, _, window, cx| {
                            window.push_notification("This is a notification.", cx)
                        })),
                ),
            )
            .child(
                section("Notification with Type")
                    .child(
                        Button::new("show-notify-info")
                            .info()
                            .label("Info")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Info,
                                        "You have been saved file successfully.",
                                    ),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-error")
                            .danger()
                            .label("Error")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Error,
                                        "There have some error occurred. Please try again later.",
                                    ),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-success")
                            .success()
                            .label("Success")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Success,
                                        "We have received your payment successfully.",
                                    ),
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("show-notify-warning")
                            .warning()
                            .label("Warning")
                            .on_click(cx.listener(|_, _, window, cx| {
                                struct WarningNotification;
                                window.push_notification(
                                    Notification::warning(
                                        "The network is not stable, please check your connection.",
                                    )
                                    .id1::<WarningNotification>("test"),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("With title and action").child(
                    Button::new("show-notify-with-title")
                        .label("Notification with Title")
                        .on_click(cx.listener(|_, _, window, cx| {
                            struct TestNotification;

                            window.push_notification(
                                Notification::new("There was a problem with your request.")
                                    .id::<TestNotification>()
                                    .title("Uh oh! Something went wrong.")
                                    .autohide(false)
                                    .action(|_, cx| {
                                        Button::new("try-again").label("Try again").on_click(
                                            cx.listener(|this, _, window, cx| {
                                                println!("You have clicked the try again action.");
                                                this.dismiss(window, cx);
                                            }),
                                        )
                                    })
                                    .on_click(cx.listener(|_, _, _, cx| {
                                        println!("Notification clicked");
                                        cx.notify();
                                    })),
                                cx,
                            )
                        })),
                ),
            )
    }
}
