use std::{borrow::Cow, rc::Rc};

use chrono::{Datelike, Local, NaiveDate};
use gpui::{
    App, ClickEvent, Context, Div, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, SharedString, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, prelude::FluentBuilder as _, px,
    relative,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Disableable as _, IconName, Selectable, Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

use super::utils::days_in_month;

/// Events emitted by the calendar.
pub enum CalendarEvent {
    /// The user selected a date.
    Selected(Date),
}

/// The date of the calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Date {
    Single(Option<NaiveDate>),
    Range(Option<NaiveDate>, Option<NaiveDate>),
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(Some(date)) => write!(f, "{}", date),
            Self::Single(None) => write!(f, "nil"),
            Self::Range(Some(start), Some(end)) => write!(f, "{} - {}", start, end),
            Self::Range(None, None) => write!(f, "nil"),
            Self::Range(Some(start), None) => write!(f, "{} - nil", start),
            Self::Range(None, Some(end)) => write!(f, "nil - {}", end),
        }
    }
}

impl From<NaiveDate> for Date {
    fn from(date: NaiveDate) -> Self {
        Self::Single(Some(date))
    }
}

impl From<(NaiveDate, NaiveDate)> for Date {
    fn from((start, end): (NaiveDate, NaiveDate)) -> Self {
        Self::Range(Some(start), Some(end))
    }
}

impl Date {
    /// Check if the date is set.
    pub fn is_some(&self) -> bool {
        match self {
            Self::Single(Some(_)) | Self::Range(Some(_), _) => true,
            _ => false,
        }
    }

    /// Check if the date is complete.
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Range(Some(_), Some(_)) => true,
            Self::Single(Some(_)) => true,
            _ => false,
        }
    }

    /// Get the start date.
    pub fn start(&self) -> Option<NaiveDate> {
        match self {
            Self::Single(Some(date)) => Some(*date),
            Self::Range(Some(start), _) => Some(*start),
            _ => None,
        }
    }

    /// Get the end date.
    pub fn end(&self) -> Option<NaiveDate> {
        match self {
            Self::Range(_, Some(end)) => Some(*end),
            _ => None,
        }
    }

    /// Return formatted date string.
    pub fn format(&self, format: &str) -> Option<SharedString> {
        match self {
            Self::Single(Some(date)) => Some(date.format(format).to_string().into()),
            Self::Range(Some(start), Some(end)) => {
                Some(format!("{} - {}", start.format(format), end.format(format)).into())
            }
            _ => None,
        }
    }

    fn is_active(&self, v: &NaiveDate) -> bool {
        let v = *v;
        match self {
            Self::Single(d) => Some(v) == *d,
            Self::Range(start, end) => Some(v) == *start || Some(v) == *end,
        }
    }

    fn is_single(&self) -> bool {
        matches!(self, Self::Single(_))
    }

    fn is_in_range(&self, v: &NaiveDate) -> bool {
        let v = *v;
        match self {
            Self::Range(start, end) => {
                if let Some(start) = start {
                    if let Some(end) = end {
                        v >= *start && v <= *end
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Day,
    Month,
    Year,
}

impl ViewMode {
    fn is_day(&self) -> bool {
        matches!(self, Self::Day)
    }

    fn is_month(&self) -> bool {
        matches!(self, Self::Month)
    }

    fn is_year(&self) -> bool {
        matches!(self, Self::Year)
    }
}

/// Matcher to match dates before and after the interval.
pub struct IntervalMatcher {
    before: Option<NaiveDate>,
    after: Option<NaiveDate>,
}

/// Matcher to match dates within the range.
pub struct RangeMatcher {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// Matcher to match dates.
pub enum Matcher {
    /// Match declare days of the week.
    ///
    /// Matcher::DayOfWeek(vec![0, 6])
    /// Will match the days of the week that are Sunday and Saturday.
    DayOfWeek(Vec<u32>),
    /// Match the included days, except for those before and after the interval.
    ///
    /// Matcher::Interval(IntervalMatcher {
    ///   before: Some(NaiveDate::from_ymd(2020, 1, 2)),
    ///   after: Some(NaiveDate::from_ymd(2020, 1, 3)),
    /// })
    /// Will match the days that are not between 2020-01-02 and 2020-01-03.
    Interval(IntervalMatcher),
    /// Match the days within the range.
    ///
    /// Matcher::Range(RangeMatcher {
    ///   from: Some(NaiveDate::from_ymd(2020, 1, 1)),
    ///   to: Some(NaiveDate::from_ymd(2020, 1, 3)),
    /// })
    /// Will match the days that are between 2020-01-01 and 2020-01-03.
    Range(RangeMatcher),
    /// Match dates using a custom function.
    ///
    /// let matcher = Matcher::Custom(Box::new(|date: &NaiveDate| {
    ///     date.day0() < 5
    /// }));
    /// Will match first 5 days of each month
    Custom(Box<dyn Fn(&NaiveDate) -> bool + Send + Sync>),
}

impl From<Vec<u32>> for Matcher {
    fn from(days: Vec<u32>) -> Self {
        Matcher::DayOfWeek(days)
    }
}

impl<F> From<F> for Matcher
where
    F: Fn(&NaiveDate) -> bool + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Matcher::Custom(Box::new(f))
    }
}

impl Matcher {
    /// Create a new interval matcher.
    pub fn interval(before: Option<NaiveDate>, after: Option<NaiveDate>) -> Self {
        Matcher::Interval(IntervalMatcher { before, after })
    }

    /// Create a new range matcher.
    pub fn range(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Matcher::Range(RangeMatcher { from, to })
    }

    /// Create a new custom matcher.
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(&NaiveDate) -> bool + Send + Sync + 'static,
    {
        Matcher::Custom(Box::new(f))
    }

    /// Check if the date matches the matcher.
    pub fn is_match(&self, date: &Date) -> bool {
        match date {
            Date::Single(Some(date)) => self.matched(date),
            Date::Range(Some(start), Some(end)) => self.matched(start) || self.matched(end),
            _ => false,
        }
    }

    fn matched(&self, date: &NaiveDate) -> bool {
        match self {
            Matcher::DayOfWeek(days) => days.contains(&date.weekday().num_days_from_sunday()),
            Matcher::Interval(interval) => {
                let before_check = interval.before.map_or(false, |before| date < &before);
                let after_check = interval.after.map_or(false, |after| date > &after);
                before_check || after_check
            }
            Matcher::Range(range) => {
                let from_check = range.from.map_or(false, |from| date < &from);
                let to_check = range.to.map_or(false, |to| date > &to);
                !from_check && !to_check
            }
            Matcher::Custom(f) => f(date),
        }
    }
}

#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    size: Size,
    state: Entity<CalendarState>,
    style: StyleRefinement,
    /// Number of the months view to show.
    number_of_months: usize,
}

/// Use to store the state of the calendar.
pub struct CalendarState {
    focus_handle: FocusHandle,
    view_mode: ViewMode,
    date: Date,
    current_year: i32,
    current_month: u8,
    years: Vec<Vec<i32>>,
    year_page: i32,
    today: NaiveDate,
    /// Number of the months view to show.
    number_of_months: usize,
    pub(crate) disabled_matcher: Option<Rc<Matcher>>,
}

impl CalendarState {
    /// Create a new calendar state.
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let today = Local::now().naive_local().date();
        Self {
            focus_handle: cx.focus_handle(),
            view_mode: ViewMode::Day,
            date: Date::Single(None),
            current_month: today.month() as u8,
            current_year: today.year(),
            years: vec![],
            year_page: 0,
            today,
            number_of_months: 1,
            disabled_matcher: None,
        }
        .year_range((today.year() - 50, today.year() + 50))
    }

    /// Set the disabled matcher of the calendar state.
    pub fn disabled_matcher(mut self, matcher: impl Into<Matcher>) -> Self {
        self.disabled_matcher = Some(Rc::new(matcher.into()));
        self
    }

    /// Set the disabled matcher of the calendar.
    ///
    /// The disabled matcher will be used to disable the days that match the matcher.
    pub fn set_disabled_matcher(
        &mut self,
        disabled: impl Into<Matcher>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.disabled_matcher = Some(Rc::new(disabled.into()));
    }

    /// Set the date of the calendar.
    ///
    /// When you set a range date, the mode will be automatically set to `Mode::Range`.
    pub fn set_date(&mut self, date: impl Into<Date>, _: &mut Window, cx: &mut Context<Self>) {
        let date = date.into();

        let invalid = self
            .disabled_matcher
            .as_ref()
            .map_or(false, |matcher| matcher.is_match(&date));

        if invalid {
            return;
        }

        self.date = date;
        match self.date {
            Date::Single(Some(date)) => {
                self.current_month = date.month() as u8;
                self.current_year = date.year();
            }
            Date::Range(Some(start), _) => {
                self.current_month = start.month() as u8;
                self.current_year = start.year();
            }
            _ => {}
        }

        cx.notify()
    }

    /// Get the date of the calendar.
    pub fn date(&self) -> Date {
        self.date
    }

    /// Set number of months to show.
    pub fn set_number_of_months(
        &mut self,
        number_of_months: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.number_of_months = number_of_months;
        cx.notify();
    }

    /// Set the year range of the calendar, default is 50 years before and after the current year.
    ///
    /// Each year page contains 20 years, so the range will be divided into chunks of 20 years is better.
    pub fn year_range(mut self, range: (i32, i32)) -> Self {
        self.years = (range.0..range.1)
            .collect::<Vec<_>>()
            .chunks(20)
            .map(|chunk| chunk.to_vec())
            .collect::<Vec<_>>();
        self.year_page = self
            .years
            .iter()
            .position(|years| years.contains(&self.current_year))
            .unwrap_or(0) as i32;
        self
    }

    /// Get year and month by offset month.
    fn offset_year_month(&self, offset_month: usize) -> (i32, u32) {
        let mut month = self.current_month as i32 + offset_month as i32;
        let mut year = self.current_year;
        while month < 1 {
            month += 12;
            year -= 1;
        }
        while month > 12 {
            month -= 12;
            year += 1;
        }

        (year, month as u32)
    }

    /// Returns the days of the month in a 2D vector to render on calendar.
    fn days(&self) -> Vec<Vec<NaiveDate>> {
        (0..self.number_of_months)
            .flat_map(|offset| {
                days_in_month(self.current_year, self.current_month as u32 + offset as u32)
            })
            .collect()
    }

    fn has_prev_year_page(&self) -> bool {
        self.year_page > 0
    }

    fn has_next_year_page(&self) -> bool {
        self.year_page < self.years.len() as i32 - 1
    }

    fn prev_year_page(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.has_prev_year_page() {
            return;
        }

        self.year_page -= 1;
        cx.notify()
    }

    fn next_year_page(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.has_next_year_page() {
            return;
        }

        self.year_page += 1;
        cx.notify()
    }

    fn prev_month(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.current_month = if self.current_month == 1 {
            12
        } else {
            self.current_month - 1
        };
        self.current_year = if self.current_month == 12 {
            self.current_year - 1
        } else {
            self.current_year
        };
        cx.notify()
    }

    fn next_month(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.current_month = if self.current_month == 12 {
            1
        } else {
            self.current_month + 1
        };
        self.current_year = if self.current_month == 1 {
            self.current_year + 1
        } else {
            self.current_year
        };
        cx.notify()
    }

    fn month_name(&self, offset_month: usize) -> SharedString {
        let (_, month) = self.offset_year_month(offset_month);
        match month {
            1 => t!("Calendar.month.January"),
            2 => t!("Calendar.month.February"),
            3 => t!("Calendar.month.March"),
            4 => t!("Calendar.month.April"),
            5 => t!("Calendar.month.May"),
            6 => t!("Calendar.month.June"),
            7 => t!("Calendar.month.July"),
            8 => t!("Calendar.month.August"),
            9 => t!("Calendar.month.September"),
            10 => t!("Calendar.month.October"),
            11 => t!("Calendar.month.November"),
            12 => t!("Calendar.month.December"),
            _ => Cow::Borrowed(""),
        }
        .into()
    }

    fn year_name(&self, offset_month: usize) -> SharedString {
        let (year, _) = self.offset_year_month(offset_month);
        year.to_string().into()
    }

    fn set_view_mode(&mut self, mode: ViewMode, _: &mut Window, cx: &mut Context<Self>) {
        self.view_mode = mode;
        cx.notify();
    }

    fn months(&self) -> Vec<SharedString> {
        [
            t!("Calendar.month.January"),
            t!("Calendar.month.February"),
            t!("Calendar.month.March"),
            t!("Calendar.month.April"),
            t!("Calendar.month.May"),
            t!("Calendar.month.June"),
            t!("Calendar.month.July"),
            t!("Calendar.month.August"),
            t!("Calendar.month.September"),
            t!("Calendar.month.October"),
            t!("Calendar.month.November"),
            t!("Calendar.month.December"),
        ]
        .iter()
        .map(|s| s.clone().into())
        .collect()
    }
}

impl Render for CalendarState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

impl Calendar {
    /// Create a new calendar element with [`CalendarState`].
    pub fn new(state: &Entity<CalendarState>) -> Self {
        Self {
            id: ("calendar", state.entity_id()).into(),
            size: Size::default(),
            state: state.clone(),
            style: StyleRefinement::default(),
            number_of_months: 1,
        }
    }

    /// Set number of months to show, default is 1.
    pub fn number_of_months(mut self, number_of_months: usize) -> Self {
        self.number_of_months = number_of_months;
        self
    }

    fn render_day(
        &self,
        d: &NaiveDate,
        offset_month: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let state = self.state.read(cx);
        let (_, month) = state.offset_year_month(offset_month);
        let day = d.day();
        let is_current_month = d.month() == month;
        let is_active = state.date.is_active(d);
        let is_in_range = state.date.is_in_range(d);

        let date = *d;
        let is_today = *d == state.today;
        let disabled = state
            .disabled_matcher
            .as_ref()
            .map_or(false, |disabled| disabled.matched(&date));

        let date_id: SharedString = format!("{}_{}", date.format("%Y-%m-%d"), offset_month).into();

        self.item_button(
            date_id.clone(),
            day.to_string(),
            is_active,
            is_in_range,
            !is_current_month || disabled,
            disabled,
            window,
            cx,
        )
        .when(is_today && !is_active, |this| {
            this.border_1().border_color(cx.theme().border)
        }) // Add border for today
        .when(!disabled, |this| {
            this.on_click(window.listener_for(
                &self.state,
                move |view, _: &ClickEvent, window, cx| {
                    if view.date.is_single() {
                        view.set_date(date, window, cx);
                        cx.emit(CalendarEvent::Selected(view.date()));
                    } else {
                        let start = view.date.start();
                        let end = view.date.end();

                        if start.is_none() && end.is_none() {
                            view.set_date(Date::Range(Some(date), None), window, cx);
                        } else if start.is_some() && end.is_none() {
                            if date < start.unwrap() {
                                view.set_date(Date::Range(Some(date), None), window, cx);
                            } else {
                                view.set_date(
                                    Date::Range(Some(start.unwrap()), Some(date)),
                                    window,
                                    cx,
                                );
                            }
                        } else {
                            view.set_date(Date::Range(Some(date), None), window, cx);
                        }

                        if view.date.is_complete() {
                            cx.emit(CalendarEvent::Selected(view.date()));
                        }
                    }
                },
            ))
        })
    }

    fn render_header(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let current_year = state.current_year;
        let view_mode = state.view_mode;
        let disabled = view_mode.is_month();
        let multiple_months = self.number_of_months > 1;
        let icon_size = match self.size {
            Size::Small => Size::Small,
            Size::Large => Size::Medium,
            _ => Size::Medium,
        };

        h_flex()
            .gap_0p5()
            .justify_between()
            .items_center()
            .child(
                Button::new("prev")
                    .icon(IconName::ArrowLeft)
                    .tab_stop(false)
                    .ghost()
                    .disabled(disabled)
                    .with_size(icon_size)
                    .when(view_mode.is_day(), |this| {
                        this.on_click(window.listener_for(&self.state, CalendarState::prev_month))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.when(!state.has_prev_year_page(), |this| this.disabled(true))
                            .on_click(
                                window.listener_for(&self.state, CalendarState::prev_year_page),
                            )
                    }),
            )
            .when(!multiple_months, |this| {
                this.child(
                    h_flex()
                        .justify_center()
                        .gap_3()
                        .child(
                            Button::new("month")
                                .ghost()
                                .label(state.month_name(0))
                                .compact()
                                .tab_stop(false)
                                .with_size(self.size)
                                .selected(view_mode.is_month())
                                .on_click(window.listener_for(
                                    &self.state,
                                    move |view, _, window, cx| {
                                        if view_mode.is_month() {
                                            view.set_view_mode(ViewMode::Day, window, cx);
                                        } else {
                                            view.set_view_mode(ViewMode::Month, window, cx);
                                        }
                                        cx.notify();
                                    },
                                )),
                        )
                        .child(
                            Button::new("year")
                                .ghost()
                                .label(current_year.to_string())
                                .compact()
                                .tab_stop(false)
                                .with_size(self.size)
                                .selected(view_mode.is_year())
                                .on_click(window.listener_for(
                                    &self.state,
                                    |view, _, window, cx| {
                                        if view.view_mode.is_year() {
                                            view.set_view_mode(ViewMode::Day, window, cx);
                                        } else {
                                            view.set_view_mode(ViewMode::Year, window, cx);
                                        }
                                        cx.notify();
                                    },
                                )),
                        ),
                )
            })
            .when(multiple_months, |this| {
                this.child(h_flex().flex_1().justify_around().children(
                    (0..self.number_of_months).map(|n| {
                        h_flex()
                            .justify_center()
                            .map(|this| match self.size {
                                Size::Small => this.gap_2(),
                                Size::Large => this.gap_4(),
                                _ => this.gap_3(),
                            })
                            .child(state.month_name(n))
                            .child(state.year_name(n))
                    }),
                ))
            })
            .child(
                Button::new("next")
                    .icon(IconName::ArrowRight)
                    .ghost()
                    .tab_stop(false)
                    .disabled(disabled)
                    .with_size(icon_size)
                    .when(view_mode.is_day(), |this| {
                        this.on_click(window.listener_for(&self.state, CalendarState::next_month))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.when(!state.has_next_year_page(), |this| this.disabled(true))
                            .on_click(
                                window.listener_for(&self.state, CalendarState::next_year_page),
                            )
                    }),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn item_button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        active: bool,
        secondary_active: bool,
        muted: bool,
        disabled: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        h_flex()
            .id(id.into())
            .map(|this| match self.size {
                Size::Small => this.size_7().rounded(cx.theme().radius / 2.),
                Size::Large => this.size_10().rounded(cx.theme().radius * 2.),
                _ => this.size_9().rounded(cx.theme().radius),
            })
            .justify_center()
            .when(muted, |this| {
                this.text_color(if disabled {
                    cx.theme().muted_foreground.opacity(0.3)
                } else {
                    cx.theme().muted_foreground
                })
            })
            .when(secondary_active, |this| {
                this.bg(if muted {
                    cx.theme().accent.opacity(0.5)
                } else {
                    cx.theme().accent
                })
                .text_color(cx.theme().accent_foreground)
            })
            .when(!active && !disabled, |this| {
                this.hover(|this| {
                    this.bg(cx.theme().accent)
                        .text_color(cx.theme().accent_foreground)
                })
            })
            .when(active, |this| {
                this.bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
            })
            .child(label.into())
    }

    fn render_days(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let weeks = [
            t!("Calendar.week.0"),
            t!("Calendar.week.1"),
            t!("Calendar.week.2"),
            t!("Calendar.week.3"),
            t!("Calendar.week.4"),
            t!("Calendar.week.5"),
            t!("Calendar.week.6"),
        ];

        h_flex()
            .map(|this| match self.size {
                Size::Small => this.gap_3().text_sm(),
                Size::Large => this.gap_5().text_base(),
                _ => this.gap_4().text_sm(),
            })
            .justify_between()
            .children(
                state
                    .days()
                    .chunks(5)
                    .enumerate()
                    .map(|(offset_month, days)| {
                        v_flex()
                            .gap_0p5()
                            .child(
                                h_flex().gap_0p5().justify_between().children(
                                    weeks
                                        .iter()
                                        .map(|week| self.render_week(week.clone(), window, cx)),
                                ),
                            )
                            .children(days.iter().map(|week| {
                                h_flex().gap_0p5().justify_between().children(
                                    week.iter()
                                        .map(|d| self.render_day(d, offset_month, window, cx)),
                                )
                            }))
                    }),
            )
    }

    fn render_week(&self, week: impl Into<SharedString>, _: &mut Window, cx: &mut App) -> Div {
        h_flex()
            .map(|this| match self.size {
                Size::Small => this.size_7().rounded(cx.theme().radius / 2.0),
                Size::Large => this.size_10().rounded(cx.theme().radius),
                _ => this.size_9().rounded(cx.theme().radius),
            })
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .text_sm()
            .child(week.into())
    }

    fn render_months(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let months = state.months();
        let current_month = state.current_month;

        h_flex()
            .mt_3()
            .gap_0p5()
            .gap_y_3()
            .map(|this| match self.size {
                Size::Small => this.mt_2().gap_y_2().w(px(208.)),
                Size::Large => this.mt_4().gap_y_4().w(px(292.)),
                _ => this.mt_3().gap_y_3().w(px(264.)),
            })
            .justify_between()
            .flex_wrap()
            .children(
                months
                    .iter()
                    .enumerate()
                    .map(|(ix, month)| {
                        let active = (ix + 1) as u8 == current_month;

                        self.item_button(
                            ix,
                            month.to_string(),
                            active,
                            false,
                            false,
                            false,
                            window,
                            cx,
                        )
                        .w(relative(0.3))
                        .text_sm()
                        .on_click(window.listener_for(
                            &self.state,
                            move |view, _, window, cx| {
                                view.current_month = (ix + 1) as u8;
                                view.set_view_mode(ViewMode::Day, window, cx);
                                cx.notify();
                            },
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
    }

    fn render_years(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let current_year = state.current_year;
        let current_page_years = &self.state.read(cx).years[state.year_page as usize].clone();

        h_flex()
            .id("years")
            .gap_0p5()
            .map(|this| match self.size {
                Size::Small => this.mt_2().gap_y_2().w(px(208.)),
                Size::Large => this.mt_4().gap_y_4().w(px(292.)),
                _ => this.mt_3().gap_y_3().w(px(264.)),
            })
            .justify_between()
            .flex_wrap()
            .children(
                current_page_years
                    .iter()
                    .enumerate()
                    .map(|(ix, year)| {
                        let year = *year;
                        let active = year == current_year;

                        self.item_button(
                            ix,
                            year.to_string(),
                            active,
                            false,
                            false,
                            false,
                            window,
                            cx,
                        )
                        .w(relative(0.2))
                        .on_click(window.listener_for(
                            &self.state,
                            move |view, _, window, cx| {
                                view.current_year = year;
                                view.set_view_mode(ViewMode::Day, window, cx);
                                cx.notify();
                            },
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
    }
}

impl Sizable for Calendar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Calendar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl EventEmitter<CalendarEvent> for CalendarState {}
impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view_mode = self.state.read(cx).view_mode;
        let number_of_months = self.number_of_months;
        self.state.update(cx, |state, _| {
            state.number_of_months = number_of_months;
        });

        v_flex()
            .id(self.id.clone())
            .track_focus(&self.state.read(cx).focus_handle)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .p_3()
            .gap_0p5()
            .refine_style(&self.style)
            .child(self.render_header(window, cx))
            .child(
                v_flex()
                    .when(view_mode.is_day(), |this| {
                        this.child(self.render_days(window, cx))
                    })
                    .when(view_mode.is_month(), |this| {
                        this.child(self.render_months(window, cx))
                    })
                    .when(view_mode.is_year(), |this| {
                        this.child(self.render_years(window, cx))
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::Date;

    #[test]
    fn test_date_to_string() {
        let date = Date::Single(Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()));
        assert_eq!(date.to_string(), "2024-08-03");

        let date = Date::Single(None);
        assert_eq!(date.to_string(), "nil");

        let date = Date::Range(
            Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()),
            Some(NaiveDate::from_ymd_opt(2024, 8, 5).unwrap()),
        );
        assert_eq!(date.to_string(), "2024-08-03 - 2024-08-05");

        let date = Date::Range(Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()), None);
        assert_eq!(date.to_string(), "2024-08-03 - nil");

        let date = Date::Range(None, Some(NaiveDate::from_ymd_opt(2024, 8, 5).unwrap()));
        assert_eq!(date.to_string(), "nil - 2024-08-05");

        let date = Date::Range(None, None);
        assert_eq!(date.to_string(), "nil");
    }
}
