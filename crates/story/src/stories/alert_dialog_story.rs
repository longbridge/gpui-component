use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};

use gpui_component::{ActiveTheme, Icon, IconName, WindowExt as _, button::Button, v_flex};

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
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for AlertDialogStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AlertDialogStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("alert-dialog-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        section("AlertDialog").child(
                            Button::new("info-alert")
                                .outline()
                                .label("Show Info Alert")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Account Created")
                                            .description("Your account has been created successfully!")
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Confirmation Dialog").child(
                            Button::new("confirm-alert")
                                .outline()
                                .label("Show Confirmation")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Delete File")
                                            .description("Are you sure you want to delete this file? This action cannot be undone.")
                                            .show_cancel(true)
                                            .action_text("Delete")
                                            .cancel_text("Cancel")
                                            .on_action(|_, window, cx| {
                                                window.push_notification("File deleted", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("With Icon").child(
                            Button::new("icon-alert")
                                .outline()
                                .label("Show with Icon")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, cx| {
                                        alert
                                            .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                                            .title("Error Occurred")
                                            .description("An unexpected error has occurred. Please try again later.")
                                    });
                                })),
                        )
                    )
                    .child(
                        section("Custom Width").child(
                            Button::new("custom-width")
                                .outline()
                                .label("Custom Width (450px)")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Custom Width")
                                            .description("This alert dialog has a custom width of 500px.")
                                            .width(px(450.))
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Long Description").child(
                            Button::new("long-desc")
                                .outline()
                                .label("Long Description")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Terms and Conditions")
                                            .description("By continuing, you agree to our Terms of Service and Privacy Policy. Please read them carefully before proceeding. These terms govern your use of our services and your account.")
                                            .show_cancel(true)
                                            .action_text("I Agree")
                                            .cancel_text("Cancel")
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Destructive Action").child(
                            Button::new("destructive")
                                .outline()
                                .label("Delete Account")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Delete Account")
                                            .description("This will permanently delete your account and all associated data. This action cannot be undone.")
                                            .show_cancel(true)
                                            .action_text("Delete Forever")
                                            .cancel_text("Keep Account")
                                            .on_action(|_, window, cx| {
                                                window.push_notification("Account deletion initiated", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Session Timeout").child(
                            Button::new("session-timeout")
                                .outline()
                                .label("Session Timeout")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Session Expired")
                                            .description("Your session has expired due to inactivity. Please log in again to continue.")
                                            .action_text("Log In")
                                            .on_action(|_, window, cx| {
                                                window.push_notification("Redirecting to login...", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Network Error Retry").child(
                            Button::new("network-error")
                                .outline()
                                .label("Network Error")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Connection Failed")
                                            .description("Unable to connect to the server. Please check your internet connection and try again.")
                                            .show_cancel(true)
                                            .action_text("Retry")
                                            .cancel_text("Cancel")
                                            .on_action(|_, window, cx| {
                                                window.push_notification("Retrying connection...", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Permission Request").child(
                            Button::new("permission")
                                .outline()
                                .label("Request Permission")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Camera Permission Required")
                                            .description("This app needs access to your camera to take photos. Please allow camera access in your system settings.")
                                            .show_cancel(true)
                                            .action_text("Open Settings")
                                            .cancel_text("Not Now")
                                            .on_action(|_, window, cx| {
                                                window.push_notification("Opening system settings...", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Update Available").child(
                            Button::new("update")
                                .outline()
                                .label("Update Available")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Update Available")
                                            .description("A new version (v2.0.0) is available. This update includes new features and bug fixes.")
                                            .show_cancel(true)
                                            .action_text("Update Now")
                                            .cancel_text("Later")
                                            .on_action(|_, window, cx| {
                                                window.push_notification("Starting update...", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Keyboard Disabled").child(
                            Button::new("keyboard-disabled")
                                .outline()
                                .label("Keyboard Disabled")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Important Notice")
                                            .description("Please read this important notice carefully before proceeding.")
                                            .keyboard(false)
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Overlay Closable").child(
                            Button::new("overlay-closable")
                                .outline()
                                .label("Overlay Closable")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Overlay Closable")
                                            .description("Click outside this dialog or press ESC to close it.")
                                            .overlay_closable(true)
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Prevent Close").child(
                            Button::new("prevent-close")
                                .outline()
                                .label("Prevent Close")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Processing")
                                            .description("A process is running. Click Continue to stop it or Cancel to keep waiting.")
                                            .show_cancel(true)
                                            .action_text("Continue")
                                            .on_action(|_, window, cx| {
                                                // Return false to prevent closing
                                                window.push_notification("Cannot close: Process still running", cx);
                                                false
                                            })
                                            .on_cancel(|_, window, cx| {
                                                window.push_notification("Waiting...", cx);
                                                false
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Custom Footer").child(
                            Button::new("custom-footer")
                                .outline()
                                .label("Custom Footer")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.open_alert_dialog(cx, |alert, _, _| {
                                        alert
                                            .title("Custom Footer Layout")
                                            .description("This alert has a custom footer with reversed button order.")
                                            .show_cancel(true)
                                            .footer(|action, cancel, window, cx| {
                                                use gpui_component::h_flex;
                                                vec![
                                                    h_flex()
                                                        .gap_2()
                                                        .w_full()
                                                        .justify_between()
                                                        .child(action(window, cx))
                                                        .child(cancel(window, cx))
                                                        .into_any_element(),
                                                ]
                                            })
                                            .on_action(|_, window, cx| {
                                                window.push_notification("Action clicked", cx);
                                                true
                                            })
                                    });
                                })),
                        ),
                    ),
            )
    }
}
