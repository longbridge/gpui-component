use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, Styled, Window,
};

use gpui_component::{
    ActiveTheme, Anchor, Theme, WindowExt as _,
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
    notification::{Notification, NotificationType},
    text::markdown,
    v_flex,
};

use crate::section;

const NOTIFICATION_MARKDOWN: &str = r#"
This is a custom notification.
- List item 1
- List item 2
- [Click here](https://github.com/longbridge/gpui-component)
"#;

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

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
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
        const ANCHORS: [Anchor; 6] = [
            Anchor::TopLeft,
            Anchor::TopCenter,
            Anchor::TopRight,
            Anchor::BottomLeft,
            Anchor::BottomCenter,
            Anchor::BottomRight,
        ];

        let view = cx.entity();

        v_flex()
            .id("notification-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_3()
            .child(
                h_flex().gap_3().child(
                    Button::new("placement")
                        .outline()
                        .label(cx.theme().notification.placement.to_string())
                        .dropdown_menu(move |menu, window, cx| {
                            let menu = ANCHORS.into_iter().fold(menu, |menu, placement| {
                                menu.item(
                                    PopupMenuItem::new(placement.to_string())
                                        .checked(cx.theme().notification.placement == placement)
                                        .on_click(window.listener_for(
                                            &view,
                                            move |_, _, _, cx| {
                                                Theme::global_mut(cx).notification.placement =
                                                    placement;
                                                cx.notify();
                                            },
                                        )),
                                )
                            });

                            menu
                        }),
                ),
            )
            .child(
                section("Simple Notification").child(
                    Button::new("show-notify-0")
                        .outline()
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
                                window.push_notification(
                                    (
                                        NotificationType::Warning,
                                        "The network is not stable, please check your connection.",
                                    ),
                                    cx,
                                )
                            })),
                    ),
            )
            .child(
                section("Unique Notification").child(
                    Button::new("show-notify-unique")
                        .outline()
                        .label("Unique Notification")
                        .on_click(cx.listener(|_, _, window, cx| {
                            window.push_notification(
                                Notification::info("This is a unique notification.")
                                    .id::<NotificationStory>()
                                    .message("This is a unique notification."),
                                cx,
                            )
                        })),
                ),
            )
            .child(
                section("Unique with Key").child(
                    h_flex()
                        .gap_3()
                        .child(
                            Button::new("show-notify-unique-key0")
                                .outline()
                                .label("A Notification")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.push_notification(
                                        Notification::info("This is A unique notification.")
                                            .id1::<NotificationStory>(1),
                                        cx,
                                    )
                                })),
                        )
                        .child(
                            Button::new("show-notify-unique-key1")
                                .outline()
                                .label("B Notification")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.push_notification(
                                        Notification::info("This is B unique notification.")
                                            .id1::<NotificationStory>(2),
                                        cx,
                                    )
                                })),
                        ),
                ),
            )
            .child(
                section("With title and action").child(
                    Button::new("show-notify-with-title")
                        .outline()
                        .label("Notification with Title")
                        .on_click(cx.listener(|_, _, window, cx| {
                            struct TestNotification;

                            window.push_notification(
                                Notification::new()
                                    .id::<TestNotification>()
                                    .title("Uh oh! Something went wrong.")
                                    .message("There was a problem with your request.")
                                    .autohide(false)
                                    .action(|_, _, cx| {
                                        Button::new("try-again").primary().label("Retry").on_click(
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
            .child(
                section("Custom Notification").child(
                    Button::new("show-notify-custom")
                        .outline()
                        .label("Show Custom Notification")
                        .on_click(cx.listener(|_, _, window, cx| {
                            window.push_notification(
                                Notification::new().content(|_, _, _| {
                                    markdown(NOTIFICATION_MARKDOWN).into_any_element()
                                }),
                                cx,
                            )
                        })),
                ),
            )
            .child({
                struct ManualOpenNotification;

                section("Manual Close Notification")
                    .child(
                        Button::new("manual-open-notify")
                            .outline()
                            .label("Show")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::new()
                                        .id::<ManualOpenNotification>()
                                        .message(
                                            "You can close this notification by \
                                            clicking the Close button.",
                                        )
                                        .autohide(false),
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new("manual-close-notify")
                            .outline()
                            .label("Dismiss All")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.remove_notification::<ManualOpenNotification>(cx);
                            })),
                    )
            })
    }
}
