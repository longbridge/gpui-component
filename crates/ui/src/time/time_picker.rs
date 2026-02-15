use chrono::NaiveTime;
use gpui::{anchored, deferred, div, prelude::FluentBuilder as _, px, App, AppContext, ClickEvent, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyBinding, MouseButton, ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Window};
use rust_i18n::t;

const CONTEXT: &'static str = "TimePicker";
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("backspace", Delete, Some(CONTEXT)),
    ])
}

/// Events emitted by the TimePicker.
#[derive(Clone)]
pub enum TimePickerEvent {
    Change(Option<NaiveTime>),
}

/// Use to store the state of the time picker.
pub struct TimePickerState {
    focus_handle: FocusHandle,
    time: Option<NaiveTime>,
    open: bool,
    time_format: SharedString,
    hour: u32,
    minute: u32,
    second: u32,
    hour_input: Entity<InputState>,
    minute_input: Entity<InputState>,
    second_input: Entity<InputState>,
    updating_inputs: bool,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for TimePickerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<TimePickerEvent> for TimePickerState {}

impl TimePickerState {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hour_input = cx.new(|cx| {
            let mut state = build_time_input(TimeUnit::Hour.max_value(), _window, cx);
            state.set_value("00", _window, cx);
            state
        });
        let minute_input = cx.new(|cx| {
            let mut state = build_time_input(TimeUnit::Minute.max_value(), _window, cx);
            state.set_value("00", _window, cx);
            state
        });
        let second_input = cx.new(|cx| {
            let mut state = build_time_input(TimeUnit::Second.max_value(), _window, cx);
            state.set_value("00", _window, cx);
            state
        });

        let mut this = Self {
            focus_handle: cx.focus_handle(),
            time: None,
            open: false,
            time_format: "%H:%M:%S".into(),
            hour: 0,
            minute: 0,
            second: 0,
            hour_input,
            minute_input,
            second_input,
            updating_inputs: false,
            _subscriptions: Vec::new(),
        };

        this.register_input_subscriptions(_window, cx);
        this
    }

    pub fn time_format(mut self, format: impl Into<SharedString>) -> Self {
        self.time_format = format.into();
        self
    }

    pub fn time(&self) -> Option<NaiveTime> {
        self.time
    }

    /// Set the picker to open state
    pub fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open = open;
        if self.open {
            self.sync_inputs(window, cx);
        }
    }

    pub fn set_time(
        &mut self,
        time: Option<NaiveTime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.time = time;
        if let Some(t) = time {
            self.hour = t.hour();
            self.minute = t.minute();
            self.second = t.second();
        } else {
            self.hour = 0;
            self.minute = 0;
            self.second = 0;
        }
        self.sync_inputs(window, cx);
        cx.notify();
    }

    fn update_time(&mut self, cx: &mut Context<Self>) {
        self.time = NaiveTime::from_hms_opt(self.hour, self.minute, self.second);
        cx.notify();
    }

    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        cx.emit(TimePickerEvent::Change(self.time));
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn select_now(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let now = chrono::Local::now().time();
        self.hour = now.hour();
        self.minute = now.minute();
        self.second = now.second();
        self.time = Some(now);
        self.sync_inputs(_window, cx);
        cx.notify();
    }

    fn on_escape(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
            return;
        }

        self.focus_back_if_need(window, cx);
        self.open = false;
        cx.emit(TimePickerEvent::Change(self.time));
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
        self.set_time(None, window, cx);
        cx.emit(TimePickerEvent::Change(None));
    }

    fn toggle_picker(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            self.sync_inputs(window, cx);
        }
        cx.notify();
    }

    pub fn format_time(&self) -> Option<String> {
        self.time.map(|t| t.format(&self.time_format).to_string())
    }

    fn register_input_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let hour_input = self.hour_input.clone();
        let hour_input_sub = cx.subscribe_in(
            &self.hour_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                this.handle_input_event(TimeUnit::Hour, &hour_input, event, window, cx);
            },
        );
        let hour_step_sub = cx.subscribe_in(
            &self.hour_input,
            window,
            move |this, _, event: &NumberInputEvent, window, cx| {
                this.handle_step_event(TimeUnit::Hour, event, window, cx);
            },
        );

        let minute_input = self.minute_input.clone();
        let minute_input_sub = cx.subscribe_in(
            &self.minute_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                this.handle_input_event(TimeUnit::Minute, &minute_input, event, window, cx);
            },
        );
        let minute_step_sub = cx.subscribe_in(
            &self.minute_input,
            window,
            move |this, _, event: &NumberInputEvent, window, cx| {
                this.handle_step_event(TimeUnit::Minute, event, window, cx);
            },
        );

        let second_input = self.second_input.clone();
        let second_input_sub = cx.subscribe_in(
            &self.second_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                this.handle_input_event(TimeUnit::Second, &second_input, event, window, cx);
            },
        );
        let second_step_sub = cx.subscribe_in(
            &self.second_input,
            window,
            move |this, _, event: &NumberInputEvent, window, cx| {
                this.handle_step_event(TimeUnit::Second, event, window, cx);
            },
        );

        self._subscriptions = vec![
            hour_input_sub,
            hour_step_sub,
            minute_input_sub,
            minute_step_sub,
            second_input_sub,
            second_step_sub,
        ];
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
        self.update_time(cx);
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

impl Render for TimePickerState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
        Empty
    }
}

/// A TimePicker element.
#[derive(IntoElement)]
pub struct TimePicker {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<TimePickerState>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    size: Size,
    appearance: bool,
    disabled: bool,
}

impl Sizable for TimePicker {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for TimePicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for TimePicker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for TimePicker {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl TimePicker {
    pub fn new(state: &Entity<TimePickerState>) -> Self {
        Self {
            id: ("time-picker", state.entity_id()).into(),
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

#[derive(Clone, Copy)]
enum TimeUnit {
    Hour,
    Minute,
    Second,
}

impl RenderOnce for TimePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_focused = self.focus_handle(cx).contains_focused(window, cx);
        let (show_clean, display_title, hour_input, minute_input, second_input, is_open) = {
            let state = self.state.read(cx);
            let show_clean = self.cleanable && state.time.is_some();
            let placeholder = self
                .placeholder
                .clone()
                .unwrap_or_else(|| t!("TimePicker.placeholder", default = "Select time").into());
            let display_title = state
                .format_time()
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
            .on_action(window.listener_for(&self.state, TimePickerState::on_enter))
            .on_action(window.listener_for(&self.state, TimePickerState::on_delete))
            .when(is_open, |this| {
                this.on_action(window.listener_for(&self.state, TimePickerState::on_escape))
            })
            .flex_none()
            .w_full()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(
                div()
                    .id("time-picker-input")
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
                            window.listener_for(&self.state, TimePickerState::toggle_picker),
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
                                    this.child(clear_button(cx).on_click(
                                        window.listener_for(&self.state, TimePickerState::clean),
                                    ))
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
                                        .gap_2()
                                        .child(
                                            h_flex()
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

use chrono::Timelike;

use crate::actions::{Cancel, Confirm};
use crate::button::{Button, ButtonVariants};
use crate::input::{clear_button, Delete, InputEvent, InputState, MaskPattern, NumberInputEvent, StepAction, StepperNumberInput};
use crate::{h_flex, v_flex, ActiveTheme, Disableable, Icon, IconName, Sizable, Size, StyleSized, StyledExt};

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
