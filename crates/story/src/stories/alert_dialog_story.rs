use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, Styled, Window, div, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, StyledExt, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants},
    dialog::{
        AlertDialog, DialogAction, DialogButtonProps, DialogClose, DialogDescription, DialogFooter,
        DialogHeader, DialogTitle,
    },
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
                            .p_0()
                            .trigger(Button::new("info-alert").outline().label("Show Info Alert"))
                            .on_ok(|_, window, cx| {
                                window.push_notification("You have confirmed the alert", cx);
                                true
                            })
                            .on_cancel(|_, window, cx| {
                                window.push_notification("Ok, you canceled the alert", cx);
                                true
                            })
                            .content(|content, _, cx| {
                                content
                                    .child(DialogHeader::new().p_4().child(DialogTitle::new().child("Are you absolutely sure?")).child(
                                        DialogDescription::new().child(
                                            "This action cannot be undone. \
                                            This will permanently delete your account from our servers.",
                                        ),
                                    ))
                                    .child(DialogFooter::new()
                                        .p_4()
                                        .border_t_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().muted)
                                        .child(
                                            DialogClose::new().child(
                                                Button::new("cancel").outline().label("Cancel")
                                            )
                                        )
                                        .child(
                                            DialogAction::new().child(
                                                Button::new("ok").label("Continue").primary()
                                            )
                                        )
                                    )
                            }),
                    ),
                )
                .child(section("With open_alert_dialog").child(
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
                                            .ok_variant(ButtonVariant::Danger)
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
                    AlertDialog::new(cx).w(px(320.)).trigger(
                        Button::new("icon-alert").outline().label("Request Permission"),
                    ).on_ok(|_, window, cx| {
                        window.push_notification("Thank you for allowing network access", cx);
                        true
                    })
                    .content(|content, _, cx| {
                        content
                            .child(
                                DialogHeader::new()
                                    .items_center()
                                    .child(Icon::new(IconName::TriangleAlert).size_10().text_color(cx.theme().warning))
                                    .child(
                                        DialogTitle::new().child("Network Permission Required"),
                                    ).child(
                                        DialogDescription::new().child(
                                            "We need your permission to access the network to provide better services. \
                                            Please allow network access in your system settings.",
                                        ),
                                    ),
                            )
                            .child(
                                DialogFooter::new()
                                    .v_flex()
                                    .child(
                                        DialogAction::new().child(
                                            Button::new("agree").w_full().primary().label("Allow")
                                        )
                                    )
                                    .child(
                                        DialogClose::new().child(
                                            Button::new("disagree").w_full().outline().label("Don't Allow")
                                        )
                                    )
                            )
                    }),
                ))
                .child(
                    section("Destructive Action").child(
                        AlertDialog::new(cx)
                            .trigger(Button::new("destructive-action").outline().danger().label("Delete Account"))
                            .on_ok(|_, window, cx| {
                                window.push_notification("Your account has been deleted", cx);
                                true
                            })
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
                                            .child(
                                                DialogClose::new().child(
                                                    Button::new("cancel").flex_1().outline().label("Cancel")
                                                )
                                            )
                                            .child(
                                                DialogAction::new().child(
                                                    Button::new("delete")
                                                        .flex_1()
                                                        .outline()
                                                        .danger()
                                                        .label("Delete Forever")
                                                )
                                            )
                                    )
                            }),
                    ),
                )
                .child(section("Without Title").child(
                    Button::new("without-title").outline().label("Dialog without Title").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .confirm()
                                    .child("This is a AlertDialog with `confirm` mode.\
                                        Will have OK, CANCEL buttons.")
                            });
                        },
                    )),
                ))
                .child(section("Session Timeout").child(
                    Button::new("session-timeout").outline().label("Session Timeout").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .on_ok(|_, window, cx| {
                                        window.push_notification("Redirecting to login...", cx);
                                        true
                                    })
                                    .title("Session Expired")
                                    .description("Your session has expired due to inactivity.\
                                        Please log in again to continue.")
                                    .footer(
                                        DialogFooter::new().child(
                                            Button::new("sign-in").label("Sign in").primary().flex_1().on_click(
                                                move |_, window, cx| {
                                                    window.push_notification("Redirecting to login...", cx);
                                                    window.close_dialog(cx);
                                                },
                                            ),
                                        )
                                    )
                            });
                        },
                    )),
                ))
                .child(section("Update Available").child(
                    AlertDialog::new(cx)
                        .trigger(Button::new("update").outline().label("Update Available"))
                        .on_cancel(|_, window, cx| {
                            window.push_notification("Update postponed", cx);
                            true
                        })
                        .on_ok(|_, window, cx| {
                            window.push_notification("Starting update...", cx);
                            true
                        })
                        .content(
                        |content, _, cx| {
                            content
                                .child(DialogHeader::new().child(DialogTitle::new().child("Update Available")).child(
                                    DialogDescription::new().child(
                                        "A new version (v2.0.0) is available.\
                                                This update includes new features and bug fixes.",
                                    ),
                                ))
                                .child(
                                    DialogFooter::new()
                                        .bg(cx.theme().muted)
                                        .child(
                                            DialogClose::new().child(
                                                Button::new("later").flex_1().outline().label("Later")
                                            ),
                                        )
                                        .child(
                                            DialogAction::new().child(
                                                Button::new("update-now").flex_1().primary().label("Update Now")
                                            )
                                        )
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
                .child(section("With confirm mode").child(
                    Button::new("overlay-closable").outline().label("Confirm Mode").on_click(cx.listener(
                        |_, _, window, cx| {
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .confirm()
                                    .title("Are you sure?")
                                    .description("This is a AlertDialog with `confirm` mode.\
                                        Will have OK, CANCEL buttons.")
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
                            window.open_alert_dialog(cx, |alert, _, _| {
                                alert
                                    .title("Processing")
                                    .close_button(true)
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
        )
    }
}
