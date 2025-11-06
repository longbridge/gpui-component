use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, div, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerState},
    h_flex,
    input::{Input, InputState},
    modal::ModalButtonProps,
    select::{Select, SelectState},
    text::TextView,
    v_flex,
};

use crate::{TestAction, section};

pub struct ModalStory {
    focus_handle: FocusHandle,
    selected_value: Option<SharedString>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    date: Entity<DatePickerState>,
    select: Entity<SelectState<Vec<String>>>,
    modal_overlay: bool,
    model_show_close: bool,
    model_keyboard: bool,
    overlay_closable: bool,
}

impl super::Story for ModalStory {
    fn title() -> &'static str {
        "Modal"
    }

    fn description() -> &'static str {
        "A modal dialog"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ModalStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
        let input2 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        let select = cx.new(|cx| {
            SelectState::new(
                vec![
                    "Option 1".to_string(),
                    "Option 2".to_string(),
                    "Option 3".to_string(),
                ],
                None,
                window,
                cx,
            )
        });

        Self {
            focus_handle: cx.focus_handle(),
            selected_value: None,
            input1,
            input2,
            date,
            select,
            modal_overlay: true,
            model_show_close: true,
            model_keyboard: true,
            overlay_closable: true,
        }
    }

    fn show_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let overlay = self.modal_overlay;
        let modal_show_close = self.model_show_close;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date = self.date.clone();
        let select = self.select.clone();
        let view = cx.entity().clone();
        let keyboard = self.model_keyboard;

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("Form Modal")
                .overlay(overlay)
                .keyboard(keyboard)
                .show_close(modal_show_close)
                .overlay_closable(overlay_closable)
                .child(
                    v_flex()
                        .gap_3()
                        .child("This is a modal dialog.")
                        .child("You can put anything here.")
                        .child(Input::new(&input1))
                        .child(Select::new(&select))
                        .child(DatePicker::new(&date).placeholder("Date of Birth")),
                )
                .footer({
                    let view = view.clone();
                    let input1 = input1.clone();
                    let date = date.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("confirm").primary().label("Confirm").on_click({
                                let view = view.clone();
                                let input1 = input1.clone();
                                let date = date.clone();
                                move |_, window, cx| {
                                    window.close_modal(cx);

                                    view.update(cx, |view, cx| {
                                        view.selected_value = Some(
                                            format!(
                                                "Hello, {}, date: {}",
                                                input1.read(cx).value(),
                                                date.read(cx).date()
                                            )
                                            .into(),
                                        )
                                    });
                                }
                            }),
                            Button::new("new-modal").label("Open Other Modal").on_click(
                                move |_, window, cx| {
                                    window.open_modal(cx, move |modal, _, _| {
                                        modal
                                            .title("Other Modal")
                                            .child("This is another modal.")
                                            .min_h(px(100.))
                                            .overlay(overlay)
                                            .keyboard(keyboard)
                                            .show_close(modal_show_close)
                                            .overlay_closable(overlay_closable)
                                    });
                                },
                            ),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_modal(cx);
                                }),
                        ]
                    }
                })
        });

        self.input1.focus_handle(cx).focus(window);
    }

    fn on_action_test_action(
        &mut self,
        _: &TestAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.push_notification("You have clicked the TestAction.", cx);
    }
}

impl Focusable for ModalStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ModalStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let modal_overlay = self.modal_overlay;
        let overlay_closable = self.overlay_closable;

        div()
            .id("modal-story")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_test_action))
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Checkbox::new("modal-overlay")
                                    .label("Modal Overlay")
                                    .checked(self.modal_overlay)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.modal_overlay = !view.modal_overlay;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("overlay-closable")
                                    .label("Overlay Closable")
                                    .checked(self.overlay_closable)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.overlay_closable = !view.overlay_closable;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("modal-show-close")
                                    .label("Model Close Button")
                                    .checked(self.model_show_close)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.model_show_close = !view.model_show_close;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("modal-keyboard")
                                    .label("Keyboard")
                                    .checked(self.model_keyboard)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.model_keyboard = !view.model_keyboard;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        section("Normal Modal").child(
                            Button::new("show-modal")
                                .outline()
                                .label("Open Modal")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.show_modal(window, cx)),
                                ),
                        ),
                    )
                    .child(
                        section("Focus back test")
                            .max_w_md()
                            .child(Input::new(&self.input2))
                            .child(
                                Button::new("test-action")
                                    .outline()
                                    .label("Test Action")
                                    .flex_shrink_0()
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(Box::new(TestAction), cx);
                                    })
                                    .tooltip(
                                        "This button for test dispatch action, \
                                        to make sure when Modal close,\
                                        \nthis still can handle the action.",
                                    ),
                            ),
                    )
                    .child(
                        section("Confirm Modal").child(
                            Button::new("confirm-modal0")
                                .outline()
                                .label("Open Confirm Modal")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_modal(cx, move |modal, _, _| {
                                        modal
                                            .confirm()
                                            .overlay(modal_overlay)
                                            .overlay_closable(overlay_closable)
                                            .child("Are you sure to submit?")
                                            .on_ok(|_, window, cx| {
                                                window
                                                    .push_notification("You have pressed ok.", cx);
                                                true
                                            })
                                            .on_cancel(|_, window, cx| {
                                                window.push_notification(
                                                    "You have pressed cancel.",
                                                    cx,
                                                );
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Confirm Modal with custom buttons").child(
                            Button::new("confirm-modal1")
                                .outline()
                                .label("Custom Buttons")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_modal(cx, move |modal, _, cx| {
                                        modal
                                            .rounded_lg()
                                            .confirm()
                                            .overlay(modal_overlay)
                                            .overlay_closable(overlay_closable)
                                            .child(
                                                h_flex().gap_3()
                                                    .child(Icon::new(IconName::TriangleAlert).size_6().text_color(cx.theme().warning))
                                                    .child("Update successful, we need to restart the application.")
                                            )
                                            .button_props(
                                                ModalButtonProps::default()
                                                    .cancel_text("Later")
                                                    .cancel_variant(ButtonVariant::Secondary)
                                                    .ok_text("Restart Now")
                                                    .ok_variant(ButtonVariant::Danger),
                                            )
                                            .on_ok(|_, window, cx| {
                                                window.push_notification(
                                                    "You have pressed restart.",
                                                    cx,
                                                );
                                                true
                                            })
                                            .on_cancel(|_, window, cx| {
                                                window.push_notification(
                                                    "You have pressed later.",
                                                    cx,
                                                );
                                                true
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Alert Modal").child(
                            Button::new("alert-modal")
                                .outline()
                                .label("Alert")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_modal(cx, move |modal, _, _| {
                                        modal
                                            .confirm()
                                            .overlay(modal_overlay)
                                            .overlay_closable(overlay_closable)
                                            .child("You are successfully logged in.")
                                            .alert()
                                            .on_close(|_, window, cx| {
                                                window
                                                    .push_notification("You have pressed Ok.", cx);
                                            })
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Scrollable Modal").child(
                            Button::new("scrollable-modal")
                                .outline()
                                .label("Scrollable Modal")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_modal(cx, move |modal, window, cx| {
                                        modal
                                            .h(px(450.))
                                            .overlay(modal_overlay)
                                            .overlay_closable(overlay_closable)
                                            .title("Modal with scrollbar")
                                            .child(TextView::markdown(
                                                "markdown1",
                                                include_str!("../../../README.md"),
                                                window,
                                                cx
                                            ))
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Custom Paddings").child(
                            Button::new("custom-modal-paddings")
                                .outline()
                                .label("Custom Paddings")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_modal(cx, move |modal, _, _| {
                                        modal
                                            .p_3()
                                            .title("Custom Modal Title")
                                            .child("This is a custom modal content, we can use paddings to control the layout and spacing within the modal.")
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Custom Style").child(
                            Button::new("custom-modal-style")
                                .outline()
                                .label("Custom Modal Style")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_modal(cx, move |modal, _, cx| {
                                        modal
                                            .rounded_lg()
                                            .bg(cx.theme().cyan)
                                            .text_color(cx.theme().info_foreground)
                                            .title("Custom Modal Title")
                                            .child("This is a custom modal content.")
                                    });
                                })),
                        ),
                    ),
            )
    }
}
