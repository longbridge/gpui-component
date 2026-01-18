use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, div, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _},
    checkbox::Checkbox,
    date_picker::{DatePicker, DatePickerState},
    dialog::DialogButtonProps,
    h_flex,
    input::{Input, InputState},
    select::{Select, SelectState},
    table::{Column, Table, TableDelegate, TableState},
    text::markdown,
    v_flex,
};

use crate::{TestAction, section};

pub struct DialogStory {
    focus_handle: FocusHandle,
    selected_value: Option<SharedString>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    date: Entity<DatePickerState>,
    select: Entity<SelectState<Vec<String>>>,
    table: Entity<TableState<MyTable>>,
    dialog_overlay: bool,
    close_button: bool,
    keyboard: bool,
    overlay_closable: bool,
}

struct MyTable {
    columns: Vec<Column>,
}
impl MyTable {
    fn new(_: &mut App) -> Self {
        let columns = vec![
            Column::new("id", "ID").width(px(50.)),
            Column::new("name", "Name").width(px(150.)),
            Column::new("email", "Email").width(px(250.)),
            Column::new("role", "Role").width(px(150.)),
            Column::new("status", "Status").width(px(100.)),
        ];

        Self { columns }
    }
}
impl TableDelegate for MyTable {
    fn columns_count(&self, _: &App) -> usize {
        5
    }

    fn rows_count(&self, _: &App) -> usize {
        200
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        match col_ix {
            0 => format!("{}", row_ix).into_any_element(),
            1 => format!("User {}", row_ix).into_any_element(),
            2 => format!("user-{}@mail.com", row_ix).into_any_element(),
            3 => "User".into_any_element(),
            4 => "Active".into_any_element(),
            _ => panic!("Invalid column index"),
        }
    }
}

impl super::Story for DialogStory {
    fn title() -> &'static str {
        "Dialog"
    }

    fn description() -> &'static str {
        "A dialog dialog"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DialogStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
        let input2 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("For test focus back on dialog close.")
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

        let table = cx.new(|cx| TableState::new(MyTable::new(cx), window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            selected_value: None,
            input1,
            input2,
            date,
            select,
            dialog_overlay: true,
            close_button: true,
            keyboard: true,
            overlay_closable: true,
            table,
        }
    }

    fn show_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let overlay = self.dialog_overlay;
        let dialog_close_button = self.close_button;
        let overlay_closable = self.overlay_closable;
        let input1 = self.input1.clone();
        let date = self.date.clone();
        let select = self.select.clone();
        let view = cx.entity().clone();
        let keyboard = self.keyboard;

        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title("Form Dialog")
                .overlay(overlay)
                .keyboard(keyboard)
                .close_button(dialog_close_button)
                .overlay_closable(overlay_closable)
                .child(
                    v_flex()
                        .gap_3()
                        .child("This is a dialog dialog.")
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
                                    window.close_dialog(cx);

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
                            Button::new("new-dialog")
                                .label("Open Other Dialog")
                                .on_click(move |_, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .title("Other Dialog")
                                            .child("This is another dialog.")
                                            .min_h(px(100.))
                                            .overlay(overlay)
                                            .keyboard(keyboard)
                                            .close_button(dialog_close_button)
                                            .overlay_closable(overlay_closable)
                                    });
                                }),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        ]
                    }
                })
        });

        self.input1.focus_handle(cx).focus(window, cx);
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

impl Focusable for DialogStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DialogStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_overlay = self.dialog_overlay;
        let overlay_closable = self.overlay_closable;

        div()
            .id("dialog-story")
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
                                Checkbox::new("dialog-overlay")
                                    .label("Dialog Overlay")
                                    .checked(self.dialog_overlay)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.dialog_overlay = !view.dialog_overlay;
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
                                Checkbox::new("dialog-show-close")
                                    .label("Model Close Button")
                                    .checked(self.close_button)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.close_button = !view.close_button;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Checkbox::new("dialog-keyboard")
                                    .label("Keyboard")
                                    .checked(self.keyboard)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.keyboard = !view.keyboard;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        section("Normal Dialog").child(
                            Button::new("show-dialog")
                                .outline()
                                .label("Open Dialog")
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.show_dialog(window, cx)),
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
                                        to make sure when Dialog close,\
                                        \nthis still can handle the action.",
                                    ),
                            ),
                    )
                    .child(
                        section("Confirm Dialog").child(
                            Button::new("confirm-dialog0")
                                .outline()
                                .label("Open Confirm Dialog")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .confirm()
                                            .overlay(dialog_overlay)
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
                        section("Confirm Dialog with custom buttons").child(
                            Button::new("confirm-dialog1")
                                .outline()
                                .label("Custom Buttons")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, cx| {
                                        dialog
                                            .rounded_lg()
                                            .confirm()
                                            .overlay(dialog_overlay)
                                            .overlay_closable(overlay_closable)
                                            .child(
                                                h_flex().gap_3()
                                                    .child(Icon::new(IconName::TriangleAlert).size_6().text_color(cx.theme().warning))
                                                    .child("Update successful, we need to restart the application.")
                                            )
                                            .button_props(
                                                DialogButtonProps::default()
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
                        section("Alert Dialog").child(
                            Button::new("alert-dialog")
                                .outline()
                                .label("Alert")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .confirm()
                                            .overlay(dialog_overlay)
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
                        section("Scrollable Dialog").child(
                            Button::new("scrollable-dialog")
                                .outline()
                                .label("Scrollable Dialog")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .w(px(720.))
                                            .h(px(600.))
                                            .overlay(dialog_overlay)
                                            .overlay_closable(overlay_closable)
                                            .title("Dialog with scrollbar")
                                            .child(markdown(
                                                include_str!("../../../../README.md"),
                                            ))
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Table in Dialog")
                            .child(
                                Button::new("table-dialog")
                                    .outline()
                                    .label("Table Dialog")
                                    .on_click(cx.listener({

                                        move |this, _, window, cx| {
                                            window.open_dialog(cx, {
                                                let table = this.table.clone();
                                                move |dialog, _, _| {
                                                    dialog
                                                        .w(px(800.))
                                                        .h(px(600.))
                                                        .overlay(dialog_overlay)
                                                        .overlay_closable(overlay_closable)
                                                        .title("Dialog with Table")
                                                        .child(
                                                            v_flex().size_full().gap_3().child(
                                                                "This is a dialog contains a table component."
                                                            )
                                                            .child(Table::new(&table))
                                                        )
                                                }
                                            });
                                        }
                                    })),
                            )
                    )
                    .child(
                        section("Custom Paddings").child(
                            Button::new("custom-dialog-paddings")
                                .outline()
                                .label("Custom Paddings")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, _| {
                                        dialog
                                            .p_3()
                                            .title("Custom Dialog Title")
                                            .child("This is a custom dialog content, we can use paddings to control the layout and spacing within the dialog.")
                                    });
                                })),
                        ),
                    )
                    .child(
                        section("Custom Style").child(
                            Button::new("custom-dialog-style")
                                .outline()
                                .label("Custom Dialog Style")
                                .on_click(cx.listener(move |_, _, window, cx| {
                                    window.open_dialog(cx, move |dialog, _, cx| {
                                        dialog
                                            .rounded_lg()
                                            .bg(cx.theme().cyan)
                                            .text_color(cx.theme().info_foreground)
                                            .title("Custom Dialog Title")
                                            .child("This is a custom dialog content.")
                                    });
                                })),
                        ),
                    ),
            )
    }
}
