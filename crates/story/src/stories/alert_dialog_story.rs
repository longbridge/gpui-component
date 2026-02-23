use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, StyledExt, WindowExt as _,
    button::{Button, ButtonVariants},
    dialog::{AlertDialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle},
    v_flex,
};

use crate::section;

pub struct AlertDialogStory {
    focus_handle: FocusHandle,
}

impl super::Story for AlertDialogStory {
    fn title() -> &'static str {
        "AlertDialog"
    }

    fn description() -> &'static str {
        "A modal dialog that interrupts the user with important content"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl AlertDialogStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }
}

impl Focusable for AlertDialogStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AlertDialogStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().id("alert-dialog-story").track_focus(&self.focus_handle).size_full().child(
            v_flex()
                .gap_6()
                .child(
                    section("AlertDialog").child(
                        AlertDialog::new(cx)
                            .trigger(Button::new("info-alert").outline().label("Show Info Alert"))
                            .title("Account Created")
                            .description("Your account has been created successfully!"),
                    ),
                )
                .child(section("Confirmation Dialog").child(
                    Button::new("confirm-alert").outline().label("Show Confirmation").on_click(cx.listener(
                        |_, _, window, cx| {
                            use gpui_component::dialog::DialogButtonProps;

                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Delete File")
                                    .description(
                                        "Are you sure you want to delete this file? \
                                                This action cannot be undone.",
                                    )
                                    .button_props(
                                        DialogButtonProps::default()
                                            .ok_text("Delete")
                                            .cancel_text("Cancel")
                                            .show_cancel(true),
                                    )
                                    .on_ok(|_, window, cx| {
                                        window.push_notification("File deleted", cx);
                                        true
                                    })
                            });
                        },
                    )),
                ))
                .child(section("With Icon").child(
                    Button::new("icon-alert").outline().label("Show with Icon").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, cx| {
                                alert
                                    .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                                    .title("Error Occurred")
                                    .description(
                                        "An unexpected error has occurred. \
                                                Please try again later.",
                                    )
                            });
                        },
                    )),
                ))
                .child(section("Custom Width").child(
                    Button::new("custom-width").outline().label("Custom Width (450px)").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Custom Width")
                                    .description("This alert dialog has a custom width of 500px.")
                                    .width(px(450.))
                            });
                        },
                    )),
                ))
                .child(section("Long Description").child(
                    AlertDialog::new(cx).trigger(Button::new("long-desc").outline().label("Long Description")).content(
                        |content, _, _| {
                            content
                                .child(
                                    DialogHeader::new().child(DialogTitle::new().child("Terms and Conditions")).child(
                                        DialogDescription::new().child(
                                            "By continuing, you agree to our Terms of Service \
                                                    and Privacy Policy. Please read them carefully \
                                                    before proceeding. These terms govern your use of \
                                                    our services and your account.",
                                        ),
                                    ),
                                )
                                .child(
                                    DialogFooter::new()
                                        .v_flex()
                                        .child(Button::new("agree").w_full().primary().label("I Agree").on_click(
                                            |_, window, cx| {
                                                window.push_notification("You agreed to the terms", cx);
                                                window.close_dialog(cx);
                                            },
                                        ))
                                        .child(
                                            Button::new("disagree").w_full().outline().label("I Disagree").on_click(
                                                |_, window, cx| {
                                                    window.push_notification("You disagreed with the terms", cx);
                                                    window.close_dialog(cx);
                                                },
                                            ),
                                        ),
                                )
                        },
                    ),
                ))
                .child(
                    section("Destructive Action").child(
                        AlertDialog::new(cx)
                            .trigger(Button::new("destructive-action").outline().danger().label("Delete Account"))
                            .content(|content, _, _| {
                                content
                                    .child(DialogHeader::new().child(DialogTitle::new().child("Delete Account")).child(
                                        DialogDescription::new().child(
                                            "This will permanently delete your account \
                                                    and all associated data. This action cannot be undone.",
                                        ),
                                    ))
                                    .child(
                                        DialogFooter::new()
                                            .child(Button::new("cancel").flex_1().outline().label("Cancel").on_click(
                                                |_, window, cx| {
                                                    window.close_dialog(cx);
                                                },
                                            ))
                                            .child(
                                                Button::new("delete")
                                                    .flex_1()
                                                    .outline()
                                                    .danger()
                                                    .label("Delete Forever")
                                                    .on_click(|_, window, cx| {
                                                        window.push_notification("Account deletion initiated", cx);
                                                        window.close_dialog(cx);
                                                    }),
                                            ),
                                    )
                            }),
                    ),
                )
                .child(section("Session Timeout").child(
                    Button::new("session-timeout").outline().label("Session Timeout").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.content(|content, _, _| {
                                    content
                                        .child(DialogHeader::new().child(DialogTitle::new().child("Session Expired")))
                                        .child(DialogDescription::new().child(
                                            "Your session has expired due to inactivity. \
                                                            Please log in again to continue.",
                                        ))
                                        .child(DialogFooter::new().child(
                                            Button::new("sign-in").label("Sign in").primary().flex_1().on_click(
                                                move |_, window, cx| {
                                                    window.push_notification("Redirecting to login...", cx);
                                                    window.close_dialog(cx);
                                                },
                                            ),
                                        ))
                                })
                            });
                        },
                    )),
                ))
                .child(section("Network Error Retry").child(
                    Button::new("network-error").outline().label("Network Error").on_click(cx.listener(
                        |_, _, window, cx| {
                            use gpui_component::dialog::DialogButtonProps;

                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Connection Failed")
                                    .description(
                                        "Unable to connect to the server. \
                                                Please check your internet connection and try again.",
                                    )
                                    .button_props(
                                        DialogButtonProps::default()
                                            .ok_text("Retry")
                                            .cancel_text("Cancel")
                                            .show_cancel(true),
                                    )
                                    .on_ok(|_, window, cx| {
                                        window.push_notification("Retrying connection...", cx);
                                        true
                                    })
                            });
                        },
                    )),
                ))
                .child(section("Permission Request").child(
                    Button::new("permission").outline().label("Request Permission").on_click(cx.listener(
                        |_, _, window, cx| {
                            use gpui_component::dialog::DialogButtonProps;

                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Camera Permission Required")
                                    .description(
                                        "This app needs access to your camera to take photos. \
                                                Please allow camera access in your system settings.",
                                    )
                                    .button_props(
                                        DialogButtonProps::default()
                                            .ok_text("Open Settings")
                                            .cancel_text("Not Now")
                                            .show_cancel(true),
                                    )
                                    .on_ok(|_, window, cx| {
                                        window.push_notification("Opening system settings...", cx);
                                        true
                                    })
                            });
                        },
                    )),
                ))
                .child(section("Update Available").child(
                    AlertDialog::new(cx).trigger(Button::new("update").outline().label("Update Available")).content(
                        |content, _, _| {
                            content
                                .child(DialogHeader::new().child(DialogTitle::new().child("Update Available")).child(
                                    DialogDescription::new().child(
                                        "A new version (v2.0.0) is available.\
                                                This update includes new features and bug fixes.",
                                    ),
                                ))
                                .child(
                                    DialogFooter::new()
                                        .v_flex()
                                        .child(Button::new("update-now").success().label("Update Now").on_click(
                                            |_, window, cx| {
                                                window.push_notification("Starting update...", cx);
                                                window.close_dialog(cx);
                                            },
                                        ))
                                        .child(Button::new("later").outline().label("Later").on_click(
                                            |_, window, cx| {
                                                window.push_notification("Update postponed", cx);
                                                window.close_dialog(cx);
                                            },
                                        )),
                                )
                        },
                    ),
                ))
                .child(section("Keyboard Disabled").child(
                    Button::new("keyboard-disabled").outline().label("Keyboard Disabled").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Important Notice")
                                    .description(
                                        "Please read this important notice \
                                                carefully before proceeding.",
                                    )
                                    .keyboard(false)
                            });
                        },
                    )),
                ))
                .child(section("Overlay Closable").child(
                    Button::new("overlay-closable").outline().label("Overlay Closable").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Overlay Closable")
                                    .description("Click outside this dialog or press ESC to close it.")
                                    .overlay_closable(true)
                            });
                        },
                    )),
                ))
                .child(section("Prevent Close").child(
                    Button::new("prevent-close").outline().label("Prevent Close").on_click(cx.listener(
                        |_, _, window, cx| {
                            use gpui_component::dialog::DialogButtonProps;

                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Processing")
                                    .description(
                                        "A process is running. \
                                                Click Continue to stop it or Cancel to keep waiting.",
                                    )
                                    .button_props(DialogButtonProps::default().ok_text("Continue").show_cancel(true))
                                    .on_ok(|_, window, cx| {
                                        // Return false to prevent closing
                                        window.push_notification("Cannot close: Process still running", cx);
                                        false
                                    })
                                    .on_cancel(|_, window, cx| {
                                        window.push_notification("Waiting...", cx);
                                        false
                                    })
                            });
                        },
                    )),
                ))
                .child(section("Custom Footer Layout (Declarative API)").child(
                    Button::new("custom-footer").outline().label("Custom Footer").on_click(cx.listener(
                        |_, _, window, cx| {
                            use gpui_component::{
                                button::ButtonVariants as _,
                                dialog::{DialogDescription, DialogFooter, DialogHeader, DialogTitle},
                            };

                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert.content(|content, _, _cx| {
                                    content
                                        .items_center()
                                        .gap_6()
                                        .child(
                                            DialogHeader::new()
                                                .items_center()
                                                .child(
                                                    DialogTitle::new()
                                                        .text_lg()
                                                        .line_height(gpui::relative(1.4))
                                                        .text_center()
                                                        .child("Custom Footer Layout"),
                                                )
                                                .child(
                                                    DialogDescription::new()
                                                        .line_height(gpui::relative(1.6))
                                                        .text_center()
                                                        .child(
                                                            "This alert has a custom footer with reversed\
                                                                    button order using declarative API.",
                                                        ),
                                                ),
                                        )
                                        .child(
                                            DialogFooter::new()
                                                .w_full()
                                                .justify_between()
                                                .child(Button::new("action").primary().label("Action").on_click(
                                                    |_: &gpui::ClickEvent, window, cx| {
                                                        window.push_notification("Action clicked", cx);
                                                        window.close_dialog(cx);
                                                    },
                                                ))
                                                .child(Button::new("cancel").label("Cancel").on_click(
                                                    |_: &gpui::ClickEvent, window, cx| {
                                                        window.close_dialog(cx);
                                                    },
                                                )),
                                        )
                                })
                            });
                        },
                    )),
                )),
        )
    }
}
