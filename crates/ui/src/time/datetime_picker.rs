use std::rc::Rc;

use super::calendar::{Calendar, CalendarEvent, CalendarState, Date, Matcher};
use crate::actions::{Cancel, Confirm};
use crate::button::{Button, ButtonVariants};
use crate::input::{clear_button, Delete, InputEvent, InputState, MaskPattern, NumberInputEvent, StepAction, StepperNumberInput};
use crate::{h_flex, v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable, Size, StyleSized, StyledExt};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use gpui::{
    anchored, deferred, div, prelude::FluentBuilder as _, px, App, AppContext, ClickEvent, Context,
    ElementId, Empty, Entity, EventEmitter, FocusHandle, Focusable
    , InteractiveElement as _, IntoElement, KeyBinding, MouseButton,
    ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Subscription, Window,
};
use rust_i18n::t;

const CONTEXT: &'static str = "DateTimePicker";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("backspace", Delete, Some(CONTEXT)),
    ])
}

/// Events emitted by the DateTimePicker.
#[derive(Clone)]
pub enum DateTimePickerEvent {
    Change(Option<NaiveDateTime>),
}

/// Use to store the state of the datetime picker.
pub struct DateTimePickerState {
    focus_handle: FocusHandle,
    datetime: Option<NaiveDateTime>,
    open: bool,
    calendar: Entity<CalendarState>,
    datetime_format: SharedString,
    hour: u32,
    minute: u32,
    second: u32,
    hour_input: Entity<InputState>,
    minute_input: Entity<InputState>,
    second_input: Entity<InputState>,
    updating_inputs: bool,
    disabled_matcher: Option<Rc<Matcher>>,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for DateTimePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<DateTimePickerEvent> for DateTimePickerState {}

impl DateTimePickerState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let calendar = cx.new(|cx| {
            let mut this = CalendarState::new(window, cx);
            this.set_date(Date::Single(None), window, cx);
            this
        });

        let calendar_subscription = cx.subscribe_in(
            &calendar,
            window,
            |this, _, ev: &CalendarEvent, window, cx| match ev {
                CalendarEvent::Selected(date) => {
                    if let Date::Single(Some(d)) = date {
                        this.update_date(*d, window, cx);
                    }
                }
            },
        );

        let hour_input = cx.new(|cx| {
            let mut state = build_time_input(TimeUnit::Hour.max_value(), window, cx);
            state.set_value("00", window, cx);
            state
        });
        let minute_input = cx.new(|cx| {
            let mut state = build_time_input(TimeUnit::Minute.max_value(), window, cx);
            state.set_value("00", window, cx);
            state
        });
        let second_input = cx.new(|cx| {
            let mut state = build_time_input(TimeUnit::Second.max_value(), window, cx);
            state.set_value("00", window, cx);
            state
        });

        let mut this = Self {
            focus_handle: cx.focus_handle(),
            datetime: None,
            calendar,
            open: false,
            datetime_format: "%Y-%m-%d %H:%M:%S".into(),
            hour: 0,
            minute: 0,
            second: 0,
            hour_input,
            minute_input,
            second_input,
            updating_inputs: false,
            disabled_matcher: None,
            _subscriptions: Vec::new(),
        };

        let mut subscriptions = vec![calendar_subscription];
        this.register_input_subscriptions(window, cx, &mut subscriptions);
        this._subscriptions = subscriptions;
        this
    }

    pub fn datetime_format(mut self, format: impl Into<SharedString>) -> Self {
        self.datetime_format = format.into();
        self
    }

    pub fn datetime(&self) -> Option<NaiveDateTime> {
        self.datetime
    }

    /// Set the picker to open state
    pub fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open = open;
        if self.open {
            self.sync_inputs(window, cx);
        }
    }

    pub fn set_datetime(
        &mut self,
        datetime: Option<NaiveDateTime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.datetime = datetime;
        if let Some(dt) = datetime {
            self.hour = dt.hour();
            self.minute = dt.minute();
            self.second = dt.second();
            self.calendar.update(cx, |view, cx| {
                view.set_date(Date::Single(Some(dt.date())), window, cx);
            });
        } else {
            self.hour = 0;
            self.minute = 0;
            self.second = 0;
            self.calendar.update(cx, |view, cx| {
                view.set_date(Date::Single(None), window, cx);
            });
        }
        self.sync_inputs(window, cx);
        cx.notify();
    }

    pub fn disabled_matcher(mut self, disabled: impl Into<Matcher>) -> Self {
        self.disabled_matcher = Some(Rc::new(disabled.into()));
        self
    }

    fn update_date(&mut self, date: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        let time = match NaiveTime::from_hms_opt(self.hour, self.minute, self.second) {
            Some(time) => time,
            None => return,
        };
        self.datetime = Some(NaiveDateTime::new(date, time));
        self.calendar.update(cx, |view, cx| {
            view.set_date(Date::Single(Some(date)), window, cx);
        });
        cx.notify();
    }

    fn update_time_in_datetime(&mut self, cx: &mut Context<Self>) {
        if let Some(dt) = self.datetime {
            if let Some(time) = NaiveTime::from_hms_opt(self.hour, self.minute, self.second) {
                self.datetime = Some(NaiveDateTime::new(dt.date(), time));
            } else {
                self.datetime = None;
            }
        }
        cx.notify();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        cx.emit(DateTimePickerEvent::Change(self.datetime));
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn select_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = chrono::Local::now().naive_local();
        self.set_datetime(Some(now), window, cx);
    }

    fn set_canlendar_disabled_matcher(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let matcher = self.disabled_matcher.clone();
        self.calendar.update(cx, |state, _| {
            state.disabled_matcher = matcher;
        });
    }

    fn on_escape(&mut self, _: &Cancel, window: &mut Window , cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
            return;
        }

        self.focus_back_if_need(window, cx);
        self.open = false;
        // 关闭时发出 Change 事件，让表格结束编辑模式
        cx.emit(DateTimePickerEvent::Change(self.datetime));
        cx.notify();
    }

    fn on_enter(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.confirm(window, cx);
        } else {
            self.open = true;
            cx.notify();
        }
    }

    fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        self.clean(&ClickEvent::default(), window, cx);
    }

    fn focus_back_if_need(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }

        if let Some(focused) = window.focused(cx) {
            if focused.contains(&self.focus_handle, window) {
                self.focus_handle.focus(window, cx);
            }
        }
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.set_datetime(None, window, cx);
        cx.emit(DateTimePickerEvent::Change(None));
    }

    fn toggle_picker(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            self.sync_inputs(window, cx);
        }
        cx.notify();
    }

    pub fn format_datetime(&self) -> Option<String> {
        self.datetime
            .map(|dt| dt.format(&self.datetime_format).to_string())
    }
    fn register_input_subscriptions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        subscriptions: &mut Vec<Subscription>,
    ) {
        let hour_input = self.hour_input.clone();
        subscriptions.push(cx.subscribe_in(
            &self.hour_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                this.handle_input_event(TimeUnit::Hour, &hour_input, event, window, cx);
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &self.hour_input,
            window,
            move |this, _, event: &NumberInputEvent, window, cx| {
                this.handle_step_event(TimeUnit::Hour, event, window, cx);
            },
        ));

        let minute_input = self.minute_input.clone();
        subscriptions.push(cx.subscribe_in(
            &self.minute_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                this.handle_input_event(TimeUnit::Minute, &minute_input, event, window, cx);
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &self.minute_input,
            window,
            move |this, _, event: &NumberInputEvent, window, cx| {
                this.handle_step_event(TimeUnit::Minute, event, window, cx);
            },
        ));

        let second_input = self.second_input.clone();
        subscriptions.push(cx.subscribe_in(
            &self.second_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                this.handle_input_event(TimeUnit::Second, &second_input, event, window, cx);
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &self.second_input,
            window,
            move |this, _, event: &NumberInputEvent, window, cx| {
                this.handle_step_event(TimeUnit::Second, event, window, cx);
            },
        ));
    }

    fn handle_input_event(
        &mut self,
        unit: TimeUnit,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.updating_inputs {
            return;
        }
        let value = parse_time_unit(input.read(cx).text().to_string(), unit.max_value());
        match event {
            InputEvent::Change => {
                if let Some(value) = value {
                    self.set_unit_value(unit, value, cx);
                }
            }
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                if let Some(value) = value {
                    self.set_unit_value(unit, value, cx);
                }
                self.sync_inputs(window, cx);
            }
            _ => {}
        }
    }

    fn handle_step_event(
        &mut self,
        unit: TimeUnit,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.updating_inputs {
            return;
        }
        match event { NumberInputEvent::Step(action) => {
            let current = self.unit_value(unit);
            let max = unit.max_value();
            let next = match action {
                StepAction::Increment => current.saturating_add(1) % max,
                StepAction::Decrement => current.checked_sub(1).unwrap_or(max - 1),
            };
            self.set_unit_value(unit, next, cx);
            self.sync_inputs(window, cx);
            } 
        }
       
    }

    fn set_unit_value(&mut self, unit: TimeUnit, value: u32, cx: &mut Context<Self>) {
        match unit {
            TimeUnit::Hour => self.hour = value,
            TimeUnit::Minute => self.minute = value,
            TimeUnit::Second => self.second = value,
        }
        self.update_time_in_datetime(cx);
    }

    fn unit_value(&self, unit: TimeUnit) -> u32 {
        match unit {
            TimeUnit::Hour => self.hour,
            TimeUnit::Minute => self.minute,
            TimeUnit::Second => self.second,
        }
    }

    fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.updating_inputs = true;
        self.hour_input.update(cx, |state, cx| {
            state.set_value(format!("{:02}", self.hour), window, cx);
        });
        self.minute_input.update(cx, |state, cx| {
            state.set_value(format!("{:02}", self.minute), window, cx);
        });
        self.second_input.update(cx, |state, cx| {
            state.set_value(format!("{:02}", self.second), window, cx);
        });
        self.updating_inputs = false;
    }
}

impl Render for DateTimePickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

/// A DateTimePicker element.
#[derive(IntoElement)]
pub struct DateTimePicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<DateTimePickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    appearance: bool,
    disabled: bool,
}

impl Sizable for DateTimePicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for DateTimePicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for DateTimePicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for DateTimePicker {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl DateTimePicker {
    pub fn new(state: &Entity<DateTimePickerState>) -> Self {
        Self {
            id: ("datetime-picker", state.entity_id()).into(),
            state: state.clone(),
            cleanable: false,
            placeholder: None,
            size: Size::default(),
            style: StyleRefinement::default(),
            appearance: true,
            disabled: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for DateTimePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |state, cx| {
            state.set_canlendar_disabled_matcher(window, cx);
        });

        let is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let (show_clean, display_title, hour_input, minute_input, second_input, is_open) = {
            let state = self.state.read(cx);
            let show_clean = self.cleanable && state.datetime.is_some();
            let placeholder = self.placeholder.clone().unwrap_or_else(|| {
                t!(
                    "DateTimePicker.placeholder",
                    default = "Select date and time"
                )
                .into()
            });
            let display_title = state
                .format_datetime()
                .unwrap_or_else(|| placeholder.to_string());

            (
                show_clean,
                display_title,
                state.hour_input.clone(),
                state.minute_input.clone(),
                state.second_input.clone(),
                state.open,
            )
        };

        if is_open {
            self.state.update(cx, |state, cx| {
                let should_sync = state.hour_input.read(cx).text().len() == 0
                    || state.minute_input.read(cx).text().len() == 0
                    || state.second_input.read(cx).text().len() == 0;
                if should_sync {
                    state.sync_inputs(window, cx);
                }
            });
        }

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .on_action(window.listener_for(&self.state, DateTimePickerState::on_enter))
            .on_action(window.listener_for(&self.state, DateTimePickerState::on_delete))
            .when(is_open, |this| {
                this.on_action(window.listener_for(&self.state, DateTimePickerState::on_escape))
            })
            .flex_none()
            .w_full()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(
                div()
                    .id("datetime-picker-input")
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(self.appearance, |this| {
                        this.bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius)
                            .when(cx.theme().shadow, |this| this.shadow_xs())
                            .when(is_focused, |this| this.focused_border(cx))
                            .when(self.disabled, |this| {
                                this.bg(cx.theme().muted)
                                    .text_color(cx.theme().muted_foreground)
                            })
                    })
                    .overflow_hidden()
                    .input_text_size(self.size)
                    .input_size(self.size)
                    .when(!is_open && !self.disabled, |this| {
                        this.on_click(
                            window.listener_for(&self.state, DateTimePickerState::toggle_picker),
                        )
                    })
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_1()
                            .child(div().w_full().overflow_hidden().child(display_title))
                            .when(!self.disabled, |this| {
                                this.when(show_clean, |this| {
                                    this.child(
                                        clear_button(cx).on_click(
                                            window.listener_for(
                                                &self.state,
                                                DateTimePickerState::clean,
                                            ),
                                        ),
                                    )
                                })
                                .when(!show_clean, |this| {
                                    this.child(
                                        Icon::new(IconName::Calendar)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                })
                            }),
                    ),
            )
            .when(is_open, |this| {
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
                                .bg(cx.theme().popover)
                                .text_color(cx.theme().popover_foreground)
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    window.listener_for(&self.state, |view, _, window, cx| {
                                        view.on_escape(&Cancel, window, cx);
                                    }),
                                )
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap_3()
                                        .child(
                                            Calendar::new(&self.state.read(cx).calendar)
                                                .number_of_months(1)
                                                .border_0()
                                                .rounded_none()
                                                .p_0()
                                                .with_size(self.size)
                                                .w_full(),
                                        )
                                        .child(
                                            h_flex()
                                                .w_full()
                                                .gap_2()
                                                .items_center()
                                                .child(
                                                    div().flex_1().child(
                                                        StepperNumberInput::new(&hour_input)
                                                            .with_size(self.size)
                                                            .appearance(true),
                                                    ),
                                                )
                                                .child(div().flex_none().child(":"))
                                                .child(
                                                    div().flex_1().child(
                                                        StepperNumberInput::new(&minute_input)
                                                            .with_size(self.size)
                                                            .appearance(true),
                                                    ),
                                                )
                                                .child(div().flex_none().child(":"))
                                                .child(
                                                    div().flex_1().child(
                                                        StepperNumberInput::new(&second_input)
                                                            .with_size(self.size)
                                                            .appearance(true),
                                                    ),
                                                ),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .mt_3()
                                        .pt_3()
                                        .border_t_1()
                                        .border_color(cx.theme().border)
                                        .justify_between()
                                        .child(
                                            Button::new("now")
                                                .small()
                                                .ghost()
                                                .label("Now")
                                                .on_click(window.listener_for(
                                                    &self.state,
                                                    |this, _, window, cx| {
                                                        this.select_now(window, cx);
                                                    },
                                                )),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(
                                                    Button::new("ok")
                                                        .small()
                                                        .primary()
                                                        .label("OK")
                                                        .on_click(window.listener_for(
                                                            &self.state,
                                                            |this, _, window, cx| {
                                                                this.confirm(window, cx);
                                                            },
                                                        )),
                                                )
                                                .child(
                                                    Button::new("cancel")
                                                        .small()
                                                        .ghost()
                                                        .label("Cancel")
                                                        .on_click(window.listener_for(
                                                            &self.state,
                                                            |this, _, window, cx| {
                                                                this.on_escape(&Cancel, window, cx);
                                                            },
                                                        )),
                                                ),
                                        ),
                                ),
                        ),
                    )
                    .with_priority(2),
                )
            })
    }
}

#[derive(Clone, Copy)]
enum TimeUnit {
    Hour,
    Minute,
    Second,
}

fn parse_time_unit(value: String, max: u32) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = trimmed.parse::<u32>().ok()?;
    if parsed >= max {
        return None;
    }
    Some(parsed)
}

fn build_time_input(max: u32, window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
        .mask_pattern(MaskPattern::number(None))
        .validate(move |text, _| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return true;
            }
            if trimmed.chars().count() > 2 {
                return false;
            }
            trimmed
                .parse::<u32>()
                .map(|value| value < max)
                .unwrap_or(false)
        })
}

impl TimeUnit {
    fn max_value(self) -> u32 {
        match self {
            TimeUnit::Hour => 24,
            TimeUnit::Minute | TimeUnit::Second => 60,
        }
    }
}
