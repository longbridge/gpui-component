//! Typed NSH declarations over retained native date and time editors.
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, SharedString,
    Subscription, Window,
    base::CalendarView,
    component::{
        Disableable, FocusableExt, Sizable, Size,
        calendar::{Calendar, CalendarEvent, CalendarState, Date, Matcher},
        date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset, DateTime},
        time_field::{
            HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision, TimeSegment,
        },
    },
};
use nsh_core::ui::{Either, Event, JsObject, PropEnum, Props, Readonly};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::rc::Rc;

use crate::display::ComponentSize;
pub(crate) fn component_size(value: &Option<ComponentSize>) -> Result<Size, String> {
    if let Some(value) = value
        && let Either::Right(value) = value.value()
    {
        if !value.is_finite() || *value <= 0. || *value > f32::MAX as f64 {
            return Err("size must be xs, sm, md, lg or positive logical pixels".into());
        }
    }
    Ok(crate::display::size(value))
}
crate::registration::named_value! {
    /// Local civil dates/times; offsets and timezone conversion are deliberately absent.
    #[derive(Default)]
    struct DateValue(Option<Either<String, Readonly<[Option<String>; 2]>>>);
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum DatePrecision {
    Minute,
    Second,
}
impl DatePrecision {
    fn native(self) -> TimePrecision {
        match self {
            Self::Minute => TimePrecision::Minute,
            Self::Second => TimePrecision::Second,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum DateHourCycle {
    H12,
    H23,
}
impl DateHourCycle {
    fn native(self) -> HourCycle {
        match self {
            Self::H12 => HourCycle::H12,
            Self::H23 => HourCycle::H23,
        }
    }
}
#[derive(Clone, PartialEq, JsObject)]
struct DatePreset {
    label: String,
    value: DateValue,
}

// One source for the shared flat NSH fields, not a second reflection schema.
macro_rules! date_props {
    ($( $(#[$attr:meta])* $field:ident:$ty:ty,)*) => {
        #[derive(Clone, PartialEq)]
        struct DateSettings { $($field:$ty,)* }
        #[derive(Clone, PartialEq, Props)]
        struct CalendarProps { $($(#[$attr])* $field:$ty,)* }
        #[derive(Clone, PartialEq, Props)]
        struct DatePickerProps {
            $($(#[$attr])* $field:$ty,)*
            #[prop(default)] placeholder:String,
            #[prop(default)] disabled:bool,
            #[prop(default)] cleanable:bool,
            #[prop(default=true)] appearance:bool,
            #[prop(default=true)] focus_ring:bool,
            date_format:Option<String>, precision:Option<DatePrecision>,
            hour_cycle:Option<DateHourCycle>, default_time:Option<String>,
            #[prop(default)] presets:Readonly<Vec<DatePreset>>,
        }
        impl CalendarProps { fn settings(&self)->DateSettings {DateSettings{$($field:self.$field.clone(),)*}} }
        impl DatePickerProps { fn settings(&self)->DateSettings {DateSettings{$($field:self.$field.clone(),)*}} }
    }
}
date_props! {
    size:Option<ComponentSize>, #[prop(default)] value:DateValue,
    #[prop(default)] range:bool,
    #[prop(default=1)] number_of_months:usize,
    #[prop(default)] first_day_of_week:u8,
    year_range:Option<Readonly<[i32;2]>>, min:Option<String>, max:Option<String>,
    #[prop(default)] disabled_dates:Readonly<Vec<String>>,
    #[prop(default)] disabled_ranges:Readonly<Vec<Readonly<[Option<String>;2]>>>,
    #[prop(default)] disabled_weekdays:Readonly<Vec<u8>>,
}
#[derive(Clone, PartialEq, Props)]
struct TimeFieldProps {
    value: Option<String>,
    size: Option<ComponentSize>,
    precision: Option<DatePrecision>,
    hour_cycle: Option<DateHourCycle>,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    invalid: bool,
}
#[derive(Clone, PartialEq, JsObject)]
struct CalendarSnapshot {
    value: DateValue,
    year: i32,
    month: u8,
    view: CalendarViewMode,
    years: Vec<i32>,
    has_previous_year_page: bool,
    has_next_year_page: bool,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum CalendarViewMode {
    Day,
    Month,
    Year,
}
#[derive(Clone, PartialEq, JsObject)]
struct DatePickerSnapshot {
    value: DateValue,
    date: DateValue,
    complete: bool,
}
#[derive(Clone, PartialEq, JsObject)]
struct TimeFieldSnapshot {
    value: String,
    segment: TimeSegmentName,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum TimeSegmentName {
    Hour,
    Minute,
    Second,
    Period,
}
#[derive(Clone, Event)]
enum CalendarEvents {
    Change(DateValue),
    Snapshot(CalendarSnapshot),
    Error(String),
}
#[derive(Clone, Event)]
enum PickerEvents {
    Change(DateValue),
    Snapshot(DatePickerSnapshot),
    Error(String),
}
#[derive(Clone, Event)]
enum TimeEvents {
    Change(String),
    Snapshot(TimeFieldSnapshot),
    Error(String),
}

fn date(text: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| "Invalid ISO date".into())
}
fn datetime(text: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M"))
        .or_else(|_| {
            NaiveDate::parse_from_str(text, "%Y-%m-%d").map(|v| v.and_hms_opt(0, 0, 0).unwrap())
        })
        .map_err(|_| "Invalid ISO date/time".into())
}
fn time(text: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(text, "%H:%M:%S%.f")
        .or_else(|_| NaiveTime::parse_from_str(text, "%H:%M"))
        .map_err(|_| "Invalid time".into())
}
fn selection(value: &DateValue, range: bool) -> Result<DateTime, String> {
    match (range, value.value()) {
        (false, None) => Ok(DateTime::Single(None)),
        (true, None) => Ok(DateTime::Range(None, None)),
        (false, Some(Either::Left(value))) => Ok(DateTime::Single(Some(datetime(value)?))),
        (true, Some(Either::Right(values))) => {
            let [a, b] = &**values;
            let a = a.as_deref().map(datetime).transpose()?;
            let b = b.as_deref().map(datetime).transpose()?;
            if a.zip(b).is_some_and(|(a, b)| a > b) {
                return Err("Range start must not follow end".into());
            }
            Ok(DateTime::Range(a, b))
        }
        _ => Err(
            "Range value must be [start, end]; single value must be an ISO string or null".into(),
        ),
    }
}
fn is_date_only(value: &DateValue) -> bool {
    match value.value() {
        Some(Either::Left(v)) => v.len() == 10,
        Some(Either::Right(v)) => v.iter().all(|v| v.as_ref().is_none_or(|v| v.len() == 10)),
        None => false,
    }
}
fn encode_date(value: Date) -> DateValue {
    match value {
        Date::Single(v) => v.map(|v| Either::Left(v.to_string())),
        Date::Range(a, b) => Some(Either::Right(
            [a.map(|v| v.to_string()), b.map(|v| v.to_string())].into(),
        )),
    }
    .into()
}
fn encode_datetime(value: DateTime) -> DateValue {
    let fmt = |v: NaiveDateTime| v.format("%Y-%m-%dT%H:%M:%S%.f").to_string();
    match value {
        DateTime::Single(v) => v.map(|v| Either::Left(fmt(v))),
        DateTime::Range(a, b) => Some(Either::Right([a.map(fmt), b.map(fmt)].into())),
    }
    .into()
}
impl DateSettings {
    fn validate(&self) -> Result<(), String> {
        component_size(&self.size)?;
        selection(&self.value, self.range)?;
        if self.number_of_months == 0 {
            return Err("numberOfMonths must be positive".into());
        }
        if self.first_day_of_week > 6 || self.disabled_weekdays.iter().any(|v| *v > 6) {
            return Err("Weekdays must be in 0 through 6".into());
        }
        if self.year_range.as_ref().is_some_and(|years| {
            let [a, b] = **years;
            a < -262000 || b > 262000 || a >= b
        }) {
            return Err("yearRange must be an ascending [start, exclusiveEnd]".into());
        }
        let min = self.min.as_deref().map(date).transpose()?;
        let max = self.max.as_deref().map(date).transpose()?;
        if min.zip(max).is_some_and(|(a, b)| a > b) {
            return Err("min must not follow max".into());
        }
        for value in self.disabled_dates.iter() {
            date(value)?;
        }
        for values in self.disabled_ranges.iter() {
            let [a, b] = &**values;
            let a = a.as_deref().map(date).transpose()?;
            let b = b.as_deref().map(date).transpose()?;
            if a.zip(b).is_some_and(|(a, b)| a > b) {
                return Err("Disabled range start must not follow end".into());
            }
        }
        Ok(())
    }
    fn weekday(&self) -> Weekday {
        match self.first_day_of_week {
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            6 => Weekday::Sat,
            _ => Weekday::Sun,
        }
    }
    fn years(&self) -> (i32, i32) {
        self.year_range
            .as_ref()
            .map(|years| {
                let [a, b] = **years;
                (a, b)
            })
            .unwrap_or_else(|| {
                let year = chrono::Local::now().year();
                (year - 50, year + 50)
            })
    }
    fn matcher(&self) -> Matcher {
        let min = self.min.as_deref().map(date).transpose().unwrap();
        let max = self.max.as_deref().map(date).transpose().unwrap();
        let dates = self
            .disabled_dates
            .iter()
            .map(|v| date(v).unwrap())
            .collect::<Vec<_>>();
        let weekdays = self.disabled_weekdays.clone();
        let ranges = self
            .disabled_ranges
            .iter()
            .map(|values| {
                let [a, b] = &**values;
                (
                    a.as_deref().map(date).transpose().unwrap(),
                    b.as_deref().map(date).transpose().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        Matcher::custom(move |v| {
            min.is_some_and(|m| *v < m)
                || max.is_some_and(|m| *v > m)
                || dates.contains(v)
                || weekdays.contains(&(v.weekday().num_days_from_sunday() as u8))
                || ranges
                    .iter()
                    .any(|(a, b)| a.is_none_or(|a| *v >= a) && b.is_none_or(|b| *v <= b))
        })
    }
}
impl DatePickerProps {
    fn presets(&self) -> Result<Vec<DateRangePreset>, String> {
        self.presets
            .iter()
            .map(|preset| {
                let value = selection(
                    &preset.value,
                    matches!(preset.value.value(), Some(Either::Right(_))),
                )?;
                if !value.is_complete() {
                    return Err("DatePreset must have a complete date or range".into());
                }
                Ok(if is_date_only(&preset.value) {
                    match value.date() {
                        Date::Single(Some(date)) => {
                            DateRangePreset::single(preset.label.clone(), date)
                        }
                        Date::Range(Some(a), Some(b)) => {
                            DateRangePreset::range(preset.label.clone(), a, b)
                        }
                        _ => unreachable!("checked completeness"),
                    }
                } else {
                    DateRangePreset::date_time(preset.label.clone(), value)
                })
            })
            .collect()
    }
    fn validate(&self) -> Result<(), String> {
        self.settings().validate()?;
        if let Some(format) = &self.date_format {
            use std::fmt::Write;
            let sample = NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            write!(&mut String::new(), "{}", sample.format(format))
                .map_err(|_| "dateFormat contains an invalid format directive")?;
        }
        if let Some(value) = &self.default_time {
            time(value)?;
        }
        self.presets()?;
        Ok(())
    }
}
struct NativeCalendar {
    context: TypedComponentContext<CalendarEvents>,
    state: Option<Entity<CalendarState>>,
    subscription: Option<Subscription>,
    focus: Option<FocusHandle>,
    pending: Vec<String>,
}
impl NativeCalendar {
    fn new(context: TypedComponentContext<CalendarEvents>) -> Self {
        Self {
            context,
            state: None,
            subscription: None,
            focus: None,
            pending: vec!["value".into(), "yearRange".into()],
        }
    }
    fn sync(
        &mut self,
        p: &CalendarProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<CalendarState>, String> {
        let config = p.settings();
        config.validate()?;
        let first = self.state.is_none();
        if first {
            let state = cx.new(|cx| CalendarState::new(window, cx));
            let context = self.context.clone();
            self.subscription = Some(window.subscribe(&state, cx, move |_, event, _, _| {
                let CalendarEvent::Selected(v) = event;
                context.emit(CalendarEvents::Change(encode_date(*v)));
            }));
            self.focus = Some(state.read(cx).focus_handle.clone());
            self.state = Some(state);
        }
        let state = self.state.as_ref().unwrap().clone();
        if first || !self.pending.is_empty() {
            let pending = std::mem::take(&mut self.pending);
            let value = selection(&p.value, p.range)?.date();
            state.update(cx, |state, cx| {
                if (first
                    || pending
                        .iter()
                        .any(|v| matches!(v.as_str(), "value" | "range")))
                    && state.date() != value
                {
                    state.set_date(value, window, cx);
                }
                state.set_disabled_matcher_shared(Some(Rc::new(config.matcher())));
                state.set_number_of_months(p.number_of_months, window, cx);
                if first || pending.iter().any(|v| v == "yearRange") {
                    state.set_year_range(config.years(), cx);
                }
            });
        }
        Ok(state)
    }
}
impl TypedNativeComponent for NativeCalendar {
    type Props = CalendarProps;
    type Event = CalendarEvents;
    fn validate_props(&self, props: &CalendarProps) -> Result<(), nsh_core::ui::PropError> {
        props.settings().validate().map_err(nsh_core::ui::PropError)
    }
    fn props_changed(&mut self, _: &CalendarProps, fields: &[String], _: &mut Window, _: &mut App) {
        self.pending.extend_from_slice(fields);
    }
    fn render(
        &mut self,
        p: &CalendarProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        match self.sync(p, window, cx) {
            Ok(state) => Some(
                Calendar::new(&state)
                    .with_size(component_size(&p.size).unwrap())
                    .number_of_months(p.number_of_months)
                    .first_day_of_week(p.settings().weekday())
                    .into_any_element(),
            ),
            Err(error) => {
                self.context.emit(CalendarEvents::Error(error));
                None
            }
        }
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn command(
        &mut self,
        p: &CalendarProps,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = match self.sync(p, window, cx) {
            Ok(v) => v,
            Err(e) => {
                self.context.emit(CalendarEvents::Error(e));
                return;
            }
        };
        if name == "focus" {
            window.focus(self.focus.as_ref().unwrap(), cx);
            return;
        }
        let result = state.update(cx, |state, cx| -> Result<CalendarSnapshot, String> {
            match name {
                "previousMonth" => state.prev_month(),
                "nextMonth" => state.next_month(),
                "previousYearPage" => {
                    state.prev_year_page();
                }
                "nextYearPage" => {
                    state.next_year_page();
                }
                "setView" => state.set_view(match args.as_str() {
                    Some("day") => CalendarView::Day,
                    Some("month") => CalendarView::Month,
                    Some("year") => CalendarView::Year,
                    _ => return Err("setView expects day, month or year".into()),
                }),
                "selectMonth" => state.select_month(
                    args.as_u64()
                        .filter(|v| (1..=12).contains(v))
                        .ok_or("selectMonth expects 1 through 12")? as u8,
                ),
                "selectYear" => state.select_year(
                    args.as_i64()
                        .filter(|v| (-262000..=262000).contains(v))
                        .ok_or("selectYear expects a supported Gregorian year")?
                        as i32,
                ),
                "activateDate" => {
                    state.activate_date(
                        date(args.as_str().ok_or("activateDate requires a date")?)?,
                        cx,
                    );
                }
                "snapshot" => {}
                _ => return Err(format!("Unknown Calendar command: {name}")),
            }
            cx.notify();
            Ok(CalendarSnapshot {
                value: encode_date(state.date()),
                year: state.current_year(),
                month: state.current_month(),
                view: match state.view() {
                    CalendarView::Day => CalendarViewMode::Day,
                    CalendarView::Month => CalendarViewMode::Month,
                    CalendarView::Year => CalendarViewMode::Year,
                },
                years: state.years_on_page().to_vec(),
                has_previous_year_page: state.has_prev_year_page(),
                has_next_year_page: state.has_next_year_page(),
            })
        });
        self.context.emit(match result {
            Ok(v) => CalendarEvents::Snapshot(v),
            Err(e) => CalendarEvents::Error(e),
        });
        self.context.redraw();
    }
}
struct NativePicker {
    context: TypedComponentContext<PickerEvents>,
    state: Option<Entity<DatePickerState>>,
    subscription: Option<Subscription>,
    focus: Option<FocusHandle>,
    enabled: bool,
    pending: Vec<String>,
}
impl NativePicker {
    fn new(context: TypedComponentContext<PickerEvents>) -> Self {
        Self {
            context,
            state: None,
            subscription: None,
            focus: None,
            enabled: true,
            pending: vec!["value".into(), "yearRange".into()],
        }
    }
    fn sync(
        &mut self,
        p: &DatePickerProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<DatePickerState>, String> {
        p.validate()?;
        let first = self.state.is_none();
        self.enabled = !p.disabled;
        if first {
            let state = cx.new(|cx| DatePickerState::new(window, cx));
            let context = self.context.clone();
            self.subscription = Some(window.subscribe(&state, cx, move |_, event, _, _| {
                let DatePickerEvent::Change(v) = event;
                context.emit(PickerEvents::Change(encode_datetime(*v)));
            }));
            self.focus = Some(state.focus_handle(cx));
            self.state = Some(state);
        }
        let state = self.state.as_ref().unwrap().clone();
        if first || !self.pending.is_empty() {
            let fields = std::mem::take(&mut self.pending);
            let config = p.settings();
            let value = selection(&p.value, p.range)?;
            state.update(cx, |state, cx| {
                state.set_time_precision(p.precision.map(DatePrecision::native), cx);
                state.set_hour_cycle(p.hour_cycle.unwrap_or(DateHourCycle::H23).native(), cx);
                state.set_date_format(p.date_format.clone().map(SharedString::from), cx);
                state.set_first_day_of_week(config.weekday(), cx);
                state.set_disabled_matcher(Some(config.matcher()), cx);
                state.set_default_time(
                    p.default_time
                        .as_deref()
                        .map(time)
                        .transpose()
                        .unwrap()
                        .unwrap_or_default(),
                    cx,
                );
                if first
                    || fields
                        .iter()
                        .any(|v| matches!(v.as_str(), "value" | "range"))
                {
                    if is_date_only(&p.value) {
                        if state.date() != value.date() {
                            state.set_date(value.date(), window, cx);
                        }
                    } else if state.date_time() != value {
                        state.set_date_time(value, window, cx);
                    }
                }
                if first || fields.iter().any(|v| v == "yearRange") {
                    state.set_year_range(config.years(), cx);
                }
            });
        }
        Ok(state)
    }
}
impl TypedNativeComponent for NativePicker {
    type Props = DatePickerProps;
    type Event = PickerEvents;
    fn validate_props(&self, props: &DatePickerProps) -> Result<(), nsh_core::ui::PropError> {
        props.validate().map_err(nsh_core::ui::PropError)
    }
    fn props_changed(
        &mut self,
        _: &DatePickerProps,
        fields: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        self.pending.extend_from_slice(fields);
    }
    fn render(
        &mut self,
        p: &DatePickerProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        match self.sync(p, window, cx) {
            Ok(state) => Some(
                DatePicker::new(&state)
                    .with_size(component_size(&p.size).unwrap())
                    .placeholder(p.placeholder.clone())
                    .disabled(p.disabled)
                    .cleanable(p.cleanable)
                    .appearance(p.appearance)
                    .focus_ring(p.focus_ring)
                    .number_of_months(p.number_of_months)
                    .presets(p.presets().unwrap())
                    .into_any_element(),
            ),
            Err(e) => {
                self.context.emit(PickerEvents::Error(e));
                None
            }
        }
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| self.enabled)
    }
    fn command(
        &mut self,
        p: &DatePickerProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        match self.sync(p, window, cx) {
            Ok(state) => match name {
                "focus" if !p.disabled => window.focus(self.focus.as_ref().unwrap(), cx),
                "snapshot" => {
                    let state = state.read(cx);
                    self.context
                        .emit(PickerEvents::Snapshot(DatePickerSnapshot {
                            value: encode_datetime(state.date_time()),
                            date: encode_date(state.date()),
                            complete: state.date_time().is_complete(),
                        }));
                }
                _ => {}
            },
            Err(e) => self.context.emit(PickerEvents::Error(e)),
        }
    }
}
struct NativeTime {
    context: TypedComponentContext<TimeEvents>,
    state: Option<Entity<TimeFieldState>>,
    subscription: Option<Subscription>,
    focus: Option<FocusHandle>,
    enabled: bool,
    pending: Vec<String>,
}
impl NativeTime {
    fn new(context: TypedComponentContext<TimeEvents>) -> Self {
        Self {
            context,
            state: None,
            subscription: None,
            focus: None,
            enabled: true,
            pending: vec!["value".into()],
        }
    }
    fn sync(
        &mut self,
        p: &TimeFieldProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<TimeFieldState>, String> {
        component_size(&p.size)?;
        let value = p.value.as_deref().map(time).transpose()?;
        let first = self.state.is_none();
        self.enabled = !p.disabled;
        if first {
            let state = cx.new(|cx| TimeFieldState::new(window, cx));
            let context = self.context.clone();
            self.subscription = Some(window.subscribe(&state, cx, move |_, event, _, _| {
                let TimeFieldEvent::Change(v) = event;
                context.emit(TimeEvents::Change(v.format("%H:%M:%S").to_string()));
            }));
            self.focus = Some(state.focus_handle(cx));
            self.state = Some(state);
        }
        let state = self.state.as_ref().unwrap().clone();
        if first || !self.pending.is_empty() {
            let fields = std::mem::take(&mut self.pending);
            state.update(cx, |state, cx| {
                state.set_precision(
                    p.precision.unwrap_or(DatePrecision::Minute).native(),
                    window,
                    cx,
                );
                state.set_hour_cycle(
                    p.hour_cycle.unwrap_or(DateHourCycle::H23).native(),
                    window,
                    cx,
                );
                if (first || fields.iter().any(|v| v == "value"))
                    && let Some(value) = value
                    && state.time() != value
                {
                    state.set_time(value, window, cx);
                }
            });
        }
        Ok(state)
    }
}
impl TypedNativeComponent for NativeTime {
    type Props = TimeFieldProps;
    type Event = TimeEvents;
    fn validate_props(&self, props: &TimeFieldProps) -> Result<(), nsh_core::ui::PropError> {
        component_size(&props.size).map_err(nsh_core::ui::PropError)?;
        props
            .value
            .as_deref()
            .map(time)
            .transpose()
            .map_err(nsh_core::ui::PropError)?;
        Ok(())
    }
    fn props_changed(
        &mut self,
        _: &TimeFieldProps,
        fields: &[String],
        _: &mut Window,
        _: &mut App,
    ) {
        self.pending.extend_from_slice(fields);
    }
    fn render(
        &mut self,
        p: &TimeFieldProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        match self.sync(p, window, cx) {
            Ok(state) => Some(
                TimeField::new(&state)
                    .with_size(component_size(&p.size).unwrap())
                    .disabled(p.disabled)
                    .invalid(p.invalid)
                    .into_any_element(),
            ),
            Err(e) => {
                self.context.emit(TimeEvents::Error(e));
                None
            }
        }
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone().filter(|_| self.enabled)
    }
    fn command(
        &mut self,
        p: &TimeFieldProps,
        name: &str,
        _: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        match self.sync(p, window, cx) {
            Ok(state) => match name {
                "focus" if !p.disabled => window.focus(self.focus.as_ref().unwrap(), cx),
                "snapshot" => {
                    let state = state.read(cx);
                    self.context.emit(TimeEvents::Snapshot(TimeFieldSnapshot {
                        value: state.time().format("%H:%M:%S").to_string(),
                        segment: match state.selected_segment() {
                            TimeSegment::Hour => TimeSegmentName::Hour,
                            TimeSegment::Minute => TimeSegmentName::Minute,
                            TimeSegment::Second => TimeSegmentName::Second,
                            TimeSegment::Period => TimeSegmentName::Period,
                        },
                    }));
                }
                _ => {}
            },
            Err(e) => self.context.emit(TimeEvents::Error(e)),
        }
    }
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeCalendar>("gpui-component.Calendar", "Calendar", NativeCalendar::new);
    registry.register::<NativePicker>("gpui-component.DatePicker", "DatePicker", NativePicker::new);
    registry.register::<NativeTime>("gpui-component.TimeField", "TimeField", NativeTime::new);
    for name in [
        "gpui-component.Calendar",
        "gpui-component.DatePicker",
        "gpui-component.TimeField",
    ] {
        registry
            .metadata()
            .native_controlled(name, &[("change", &["value"])]);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_dates_and_partial_ranges_preserve_the_wire_contract() {
        assert!(date("2024-02-29").is_ok());
        assert!(date("2025-02-29").is_err());
        assert!(time("24:00").is_err());
        assert!(datetime("2026-10-18T09:30:00Z").is_err());
        let value = DateValue::new(Some(Either::Left("2026-10-18T09:30:00.123".into())));
        assert_eq!(
            encode_datetime(selection(&value, false).unwrap()).value(),
            value.value()
        );
        let value = DateValue::new(Some(Either::Right(
            [Some("2026-10-18".into()), None].into(),
        )));
        assert_eq!(
            encode_date(selection(&value, true).unwrap().date()).value(),
            value.value()
        );
        assert!(
            selection(
                &DateValue::new(Some(Either::Right(
                    [Some("2026-10-19".into()), Some("2026-10-18".into())].into()
                ))),
                true
            )
            .is_err()
        );
        assert!(selection(&value, false).is_err());
    }
}
