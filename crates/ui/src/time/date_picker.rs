use chrono::NaiveDate;
use gpui::{
    anchored, deferred, div, prelude::FluentBuilder as _, px, App, AppContext, Context, ElementId,
    Empty, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, Length, MouseButton, ParentElement as _, Render, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Window,
};
use rust_i18n::t;

use crate::{
    actions::Cancel,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::clear_button,
    v_flex, ActiveTheme, Icon, IconName, Sizable, Size, StyleSized as _, StyledExt as _,
};

use super::calendar::{Calendar, CalendarEvent, CalendarState, Date, Matcher};

pub(crate) fn init(cx: &mut App) {
    let context = Some("DatePicker");
    cx.bind_keys([KeyBinding::new("escape", Cancel, context)])
}

#[derive(Clone)]
pub enum DatePickerEvent {
    Change(Date),
}

#[derive(Clone)]
pub enum DateRangePresetValue {
    Single(NaiveDate),
    Range(NaiveDate, NaiveDate),
}

#[derive(Clone)]
pub struct DateRangePreset {
    label: SharedString,
    value: DateRangePresetValue,
}

impl DateRangePreset {
    /// Creates a new DateRangePreset with single date.
    pub fn single(label: impl Into<SharedString>, single: NaiveDate) -> Self {
        DateRangePreset {
            label: label.into(),
            value: DateRangePresetValue::Single(single),
        }
    }
    /// Creates a new DateRangePreset with a range of dates.
    pub fn range(label: impl Into<SharedString>, start: NaiveDate, end: NaiveDate) -> Self {
        DateRangePreset {
            label: label.into(),
            value: DateRangePresetValue::Range(start, end),
        }
    }
}

/// Use to store the state of the date picker.
pub struct DatePickerState {
    focus_handle: FocusHandle,
    date: Date,
    open: bool,
    calendar: Entity<CalendarState>,
    date_format: SharedString,
    number_of_months: usize,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for DatePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<DatePickerEvent> for DatePickerState {}

impl DatePickerState {
    /// Create a date state.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_range(false, window, cx)
    }

    /// Create a date state with range mode.
    pub fn range(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_range(true, window, cx)
    }

    fn new_with_range(is_range: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date = if is_range {
            Date::Range(None, None)
        } else {
            Date::Single(None)
        };

        let calendar = cx.new(|cx| {
            let mut this = CalendarState::new(window, cx);
            this.set_date(date, window, cx);
            this
        });

        let _subscriptions = vec![cx.subscribe_in(
            &calendar,
            window,
            |this, _, ev: &CalendarEvent, window, cx| match ev {
                CalendarEvent::Selected(date) => {
                    this.update_date(*date, true, window, cx);
                    this.focus_handle.focus(window);
                }
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            date,
            calendar,
            open: false,
            date_format: "%Y/%m/%d".into(),
            number_of_months: 1,
            _subscriptions,
        }
    }

    /// Set the date format of the date picker to display in Input, default: "%Y/%m/%d".
    pub fn date_format(mut self, format: impl Into<SharedString>) -> Self {
        self.date_format = format.into();
        self
    }

    /// Set the number of months calendar view to display, default is 1.
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = number_of_months;
        self
    }

    /// Get the date of the date picker.
    pub fn date(&self) -> Date {
        self.date
    }

    /// Set the date of the date picker.
    pub fn set_date(&mut self, date: impl Into<Date>, window: &mut Window, cx: &mut Context<Self>) {
        self.update_date(date.into(), false, window, cx);
    }

    fn update_date(&mut self, date: Date, emit: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.date = date;
        self.calendar.update(cx, |view, cx| {
            view.set_date(date, window, cx);
        });
        self.open = false;
        if emit {
            cx.emit(DatePickerEvent::Change(date));
        }
        cx.notify();
    }

    /// Set the disabled matcher of the date picker.
    pub fn set_disabled(
        &mut self,
        disabled: impl Into<Matcher>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.calendar.update(cx, |view, cx| {
            view.set_disabled(disabled.into(), window, cx);
        });
    }

    fn escape(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
        }

        self.focus_back_if_need(window, cx);
        self.open = false;

        cx.notify();
    }

    // To focus the Picker Input, if current focus in is on the container.
    //
    // This is because mouse down out the Calendar, GPUI will move focus to the container.
    // So we need to move focus back to the Picker Input.
    //
    // But if mouse down target is some other focusable element (e.g.: TextInput), we should not move focus.
    fn focus_back_if_need(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }

        if let Some(focused) = window.focused(cx) {
            if focused.contains(&self.focus_handle, window) {
                self.focus_handle.focus(window);
            }
        }
    }

    fn clean(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        match self.date {
            Date::Single(_) => {
                self.update_date(Date::Single(None), true, window, cx);
            }
            Date::Range(_, _) => {
                self.update_date(Date::Range(None, None), true, window, cx);
            }
        }
    }

    fn toggle_calendar(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    fn select_preset(
        &mut self,
        preset: &DateRangePreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match preset.value {
            DateRangePresetValue::Single(single) => {
                self.update_date(Date::Single(Some(single)), true, window, cx)
            }
            DateRangePresetValue::Range(start, end) => {
                self.update_date(Date::Range(Some(start), Some(end)), true, window, cx)
            }
        }
    }
}

#[derive(IntoElement)]
pub struct DatePicker {
    id: ElementId,
    state: Entity<DatePickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    width: Length,
    number_of_months: usize,
    presets: Option<Vec<DateRangePreset>>,
}

impl Sizable for DatePicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for DatePicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Render for DatePickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

impl DatePicker {
    pub fn new(state: &Entity<DatePickerState>) -> Self {
        Self {
            id: ("date-picker", state.entity_id()).into(),
            state: state.clone(),
            cleanable: true,
            placeholder: None,
            size: Size::default(),
            width: Length::Auto,
            number_of_months: 2,
            presets: None,
        }
    }

    /// Set the placeholder of the date picker, default: "".
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set true to show the clear button when the input field is not empty.
    pub fn cleanable(mut self) -> Self {
        self.cleanable = true;
        self
    }

    /// Set width of the date picker input field, default is `Length::Auto`.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Set preset ranges for the date picker.
    pub fn presets(mut self, presets: Vec<DateRangePreset>) -> Self {
        self.presets = Some(presets);
        self
    }

    /// Set number of months to display in the calendar, default is 2.
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = number_of_months;
        self
    }
}

impl RenderOnce for DatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // This for keep focus border style, when click on the popup.
        let is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let state = self.state.read(cx);
        let show_clean = self.cleanable && state.date.is_some();
        let placeholder = self
            .placeholder
            .clone()
            .unwrap_or_else(|| t!("DatePicker.placeholder").into());
        let display_title = state
            .date
            .format(&state.date_format)
            .unwrap_or(placeholder.clone());

        div()
            .id(self.id.clone())
            .key_context("DatePicker")
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .when(state.open, |this| {
                this.on_action(window.listener_for(&self.state, DatePickerState::escape))
            })
            .w_full()
            .relative()
            .map(|this| match self.width {
                Length::Definite(l) => this.flex_none().w(l),
                Length::Auto => this.w_full(),
            })
            .input_text_size(self.size)
            .child(
                div()
                    .id("date-picker-input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().input)
                    .rounded(cx.theme().radius)
                    .when(cx.theme().shadow, |this| this.shadow_sm())
                    .overflow_hidden()
                    .input_text_size(self.size)
                    .when(is_focused, |this| this.focused_border(cx))
                    .input_size(self.size)
                    .when(!state.open, |this| {
                        this.on_click(
                            window.listener_for(&self.state, DatePickerState::toggle_calendar),
                        )
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .child(div().w_full().overflow_hidden().child(display_title))
                            .when(show_clean, |this| {
                                this.child(clear_button(cx).on_click(
                                    window.listener_for(&self.state, DatePickerState::clean),
                                ))
                            })
                            .when(!show_clean, |this| {
                                this.child(
                                    Icon::new(IconName::Calendar)
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground),
                                )
                            }),
                    ),
            )
            .when(state.open, |this| {
                this.child(
                    deferred(
                        anchored().snap_to_window_with_margin(px(8.)).child(
                            div()
                                .occlude()
                                .mt_1p5()
                                .p_3()
                                .border_1()
                                .border_color(cx.theme().border)
                                .shadow_lg()
                                .rounded((cx.theme().radius * 2.).min(px(8.)))
                                .bg(cx.theme().background)
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    window.listener_for(&self.state, |view, _, window, cx| {
                                        view.escape(&Cancel, window, cx);
                                    }),
                                )
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .h_full()
                                        .items_start()
                                        .when_some(self.presets.clone(), |this, presets| {
                                            this.child(
                                                v_flex().my_1().gap_2().justify_end().children(
                                                    presets.into_iter().enumerate().map(
                                                        |(i, preset)| {
                                                            Button::new(("preset", i))
                                                                .small()
                                                                .ghost()
                                                                .label(preset.label.clone())
                                                                .on_click(window.listener_for(
                                                                    &self.state,
                                                                    move |this, _, window, cx| {
                                                                        this.select_preset(
                                                                            &preset, window, cx,
                                                                        );
                                                                    },
                                                                ))
                                                        },
                                                    ),
                                                ),
                                            )
                                        })
                                        .child(
                                            Calendar::new(&state.calendar)
                                                .number_of_months(self.number_of_months)
                                                .bordered(false)
                                                .with_size(self.size),
                                        ),
                                ),
                        ),
                    )
                    .with_priority(2),
                )
            })
    }
}
