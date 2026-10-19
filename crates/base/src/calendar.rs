use crate::TestSupportExt as _;
use std::rc::Rc;

use crate::{
    a11y_disabled::A11yDisabledExt as _, actions::Confirm, h_flex, styled::StyledExt as _, v_flex,
};
use chrono::{Datelike, Local, Months, NaiveDate, Weekday};
use gpui::{
    Action, AnyElement, App, Context, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, actions, div, px,
};

const CONTEXT: &str = "Calendar";

actions!(
    calendar,
    [
        PreviousDay,
        NextDay,
        PreviousWeek,
        NextWeek,
        PreviousMonth,
        NextMonth,
        PreviousYear,
        NextYear,
        WeekStart,
        WeekEnd
    ]
);

#[doc(hidden)]
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", PreviousDay, Some(CONTEXT)),
        KeyBinding::new("right", NextDay, Some(CONTEXT)),
        KeyBinding::new("up", PreviousWeek, Some(CONTEXT)),
        KeyBinding::new("down", NextWeek, Some(CONTEXT)),
        KeyBinding::new("pageup", PreviousMonth, Some(CONTEXT)),
        KeyBinding::new("pagedown", NextMonth, Some(CONTEXT)),
        KeyBinding::new("shift-pageup", PreviousYear, Some(CONTEXT)),
        KeyBinding::new("shift-pagedown", NextYear, Some(CONTEXT)),
        KeyBinding::new("home", WeekStart, Some(CONTEXT)),
        KeyBinding::new("end", WeekEnd, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
    ]);
}

/// How far a key moves the focused day. A move by days or weeks skips the
/// disabled days in its way; the others settle on the enabled day nearest
/// their target, back towards where they started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusMove {
    Days(i64),
    Months(i32),
    WeekStart,
    WeekEnd,
}

impl FocusMove {
    /// The day the move aims at from `date`, and the step that skips
    /// disabled days past it (0: none).
    fn target(self, date: NaiveDate, first_day: Weekday) -> Option<(NaiveDate, i64)> {
        let week_offset =
            (date.weekday().num_days_from_sunday() + 7 - first_day.num_days_from_sunday()) % 7;
        match self {
            Self::Days(days) => Some((date + chrono::Duration::days(days), days)),
            Self::Months(months) if months < 0 => Some((
                date.checked_sub_months(Months::new(months.unsigned_abs()))?,
                0,
            )),
            Self::Months(months) => Some((date.checked_add_months(Months::new(months as u32))?, 0)),
            Self::WeekStart => Some((date - chrono::Duration::days(week_offset as i64), 0)),
            Self::WeekEnd => Some((date + chrono::Duration::days(6 - week_offset as i64), 0)),
        }
    }
}

/// The furthest a move skips over disabled days before it gives up.
const MAX_SKIPPED_DAYS: i64 = 366;

fn month_index(year: i32, month: u32) -> i64 {
    year as i64 * 12 + month as i64 - 1
}

/// A controlled calendar value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Date {
    Single(Option<NaiveDate>),
    Range(Option<NaiveDate>, Option<NaiveDate>),
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(Some(date)) => write!(f, "{date}"),
            Self::Single(None) | Self::Range(None, None) => write!(f, "nil"),
            Self::Range(Some(start), Some(end)) => write!(f, "{start} - {end}"),
            Self::Range(Some(start), None) => write!(f, "{start} - nil"),
            Self::Range(None, Some(end)) => write!(f, "nil - {end}"),
        }
    }
}

impl From<NaiveDate> for Date {
    fn from(value: NaiveDate) -> Self {
        Self::Single(Some(value))
    }
}
impl From<(NaiveDate, NaiveDate)> for Date {
    fn from((start, end): (NaiveDate, NaiveDate)) -> Self {
        Self::Range(Some(start), Some(end))
    }
}

impl Date {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Single(Some(_)) | Self::Range(Some(_), _))
    }
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Single(Some(_)) | Self::Range(Some(_), Some(_)))
    }
    pub fn start(&self) -> Option<NaiveDate> {
        match self {
            Self::Single(Some(v)) | Self::Range(Some(v), _) => Some(*v),
            _ => None,
        }
    }
    pub fn end(&self) -> Option<NaiveDate> {
        match self {
            Self::Range(_, Some(v)) => Some(*v),
            _ => None,
        }
    }
    pub fn format(&self, format: &str) -> Option<SharedString> {
        match self {
            Self::Single(Some(v)) => Some(v.format(format).to_string().into()),
            Self::Range(Some(a), Some(b)) => {
                Some(format!("{} - {}", a.format(format), b.format(format)).into())
            }
            _ => None,
        }
    }
    pub fn is_active(&self, value: &NaiveDate) -> bool {
        match self {
            Self::Single(v) => *v == Some(*value),
            Self::Range(a, b) => *a == Some(*value) || *b == Some(*value),
        }
    }
    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single(_))
    }
    pub fn is_in_range(&self, value: &NaiveDate) -> bool {
        matches!(self, Self::Range(Some(a), Some(b)) if value >= a && value <= b)
    }
}

pub struct IntervalMatcher {
    before: Option<NaiveDate>,
    after: Option<NaiveDate>,
}
pub struct RangeMatcher {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}
pub enum Matcher {
    DayOfWeek(Vec<u32>),
    Interval(IntervalMatcher),
    Range(RangeMatcher),
    Custom(Box<dyn Fn(&NaiveDate) -> bool + Send + Sync>),
}
impl From<Vec<u32>> for Matcher {
    fn from(v: Vec<u32>) -> Self {
        Self::DayOfWeek(v)
    }
}
impl<F: Fn(&NaiveDate) -> bool + Send + Sync + 'static> From<F> for Matcher {
    fn from(v: F) -> Self {
        Self::Custom(Box::new(v))
    }
}
impl Matcher {
    pub fn interval(before: Option<NaiveDate>, after: Option<NaiveDate>) -> Self {
        Self::Interval(IntervalMatcher { before, after })
    }
    pub fn range(from: Option<NaiveDate>, to: Option<NaiveDate>) -> Self {
        Self::Range(RangeMatcher { from, to })
    }
    pub fn custom<F: Fn(&NaiveDate) -> bool + Send + Sync + 'static>(f: F) -> Self {
        Self::Custom(Box::new(f))
    }
    pub fn is_match(&self, date: &Date) -> bool {
        match date {
            Date::Single(Some(v)) => self.matched(v),
            Date::Range(Some(a), Some(b)) => self.matched(a) || self.matched(b),
            _ => false,
        }
    }
    pub fn matched(&self, date: &NaiveDate) -> bool {
        match self {
            Self::DayOfWeek(days) => days.contains(&date.weekday().num_days_from_sunday()),
            Self::Interval(v) => {
                v.before.is_some_and(|x| date < &x) || v.after.is_some_and(|x| date > &x)
            }
            Self::Range(v) => {
                !v.from.is_some_and(|x| date < &x) && !v.to.is_some_and(|x| date > &x)
            }
            Self::Custom(f) => f(date),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarView {
    Day,
    Month,
    Year,
}
impl CalendarView {
    pub fn is_day(self) -> bool {
        self == Self::Day
    }
    pub fn is_month(self) -> bool {
        self == Self::Month
    }
    pub fn is_year(self) -> bool {
        self == Self::Year
    }
}

fn picker_grid_layout(view: CalendarView) -> Option<(u16, f32)> {
    match view {
        CalendarView::Day => None,
        CalendarView::Month => Some((3, 4.)),
        CalendarView::Year => Some((5, 4.)),
    }
}

pub enum CalendarEvent {
    Selected(Date),
}

pub struct CalendarState {
    pub focus_handle: FocusHandle,
    view: CalendarView,
    date: Date,
    current_year: i32,
    current_month: u8,
    years: Vec<Vec<i32>>,
    year_page: i32,
    today: NaiveDate,
    number_of_months: usize,
    disabled_matcher: Option<Rc<Matcher>>,
    focused_date: Option<NaiveDate>,
}

impl CalendarState {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let today = Local::now().date_naive();
        Self {
            focus_handle: cx.focus_handle().tab_stop(true),
            view: CalendarView::Day,
            date: Date::Single(None),
            current_year: today.year(),
            current_month: today.month() as u8,
            years: vec![],
            year_page: 0,
            today,
            number_of_months: 1,
            disabled_matcher: None,
            focused_date: None,
        }
        .year_range((today.year() - 50, today.year() + 50))
    }
    pub fn disabled_matcher(mut self, matcher: impl Into<Matcher>) -> Self {
        self.disabled_matcher = Some(Rc::new(matcher.into()));
        self
    }
    pub fn set_disabled_matcher(
        &mut self,
        matcher: impl Into<Matcher>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.disabled_matcher = Some(Rc::new(matcher.into()));
    }
    pub fn set_disabled_matcher_shared(&mut self, matcher: Option<Rc<Matcher>>) {
        self.disabled_matcher = matcher;
    }
    pub fn disabled_matcher_ref(&self) -> Option<&Matcher> {
        self.disabled_matcher.as_deref()
    }
    pub fn set_date(&mut self, date: impl Into<Date>, _: &mut Window, cx: &mut Context<Self>) {
        if self.apply_date(date.into()) {
            cx.notify();
        }
    }
    pub fn apply_date(&mut self, date: Date) -> bool {
        if self
            .disabled_matcher
            .as_ref()
            .is_some_and(|m| m.is_match(&date))
        {
            return false;
        }
        if date != self.date
            && let Some(start) = date.start()
        {
            self.focused_date = Some(start);
        }
        self.date = date;
        if let Some(start) = date.start() {
            self.show_month_of(start);
        }
        true
    }
    pub fn select_date(&mut self, value: NaiveDate) -> bool {
        if self
            .disabled_matcher
            .as_ref()
            .is_some_and(|m| m.matched(&value))
        {
            return false;
        }
        let next = match self.date {
            Date::Single(_) => Date::Single(Some(value)),
            Date::Range(None, None) | Date::Range(None, Some(_)) => Date::Range(Some(value), None),
            Date::Range(Some(start), None) if value >= start => {
                Date::Range(Some(start), Some(value))
            }
            Date::Range(Some(_), None) | Date::Range(Some(_), Some(_)) => {
                Date::Range(Some(value), None)
            }
        };
        self.apply_date(next);
        self.focused_date = Some(value);
        self.date.is_complete()
    }
    /// Activates a day item and emits [`CalendarEvent::Selected`] once the
    /// controlled value is complete. This is the single pointer/keyboard
    /// activation path used by the calendar root.
    pub fn activate_date(&mut self, value: NaiveDate, cx: &mut Context<Self>) -> bool {
        let complete = self.select_date(value);
        if complete {
            cx.emit(CalendarEvent::Selected(self.date()));
        }
        cx.notify();
        complete
    }
    pub fn date(&self) -> Date {
        self.date
    }
    pub fn set_number_of_months(&mut self, n: usize, _: &mut Window, cx: &mut Context<Self>) {
        if self.number_of_months != n {
            self.number_of_months = n;
            cx.notify();
        }
    }
    pub fn number_of_months(&self) -> usize {
        self.number_of_months
    }
    pub fn year_range(mut self, range: (i32, i32)) -> Self {
        self.apply_year_range(range);
        self
    }
    pub fn set_year_range(&mut self, range: (i32, i32), cx: &mut Context<Self>) {
        self.apply_year_range(range);
        cx.notify();
    }
    fn apply_year_range(&mut self, range: (i32, i32)) {
        self.years = (range.0..range.1)
            .collect::<Vec<_>>()
            .chunks(20)
            .map(<[_]>::to_vec)
            .collect();
        self.year_page = self
            .years
            .iter()
            .position(|v| v.contains(&self.current_year))
            .unwrap_or(0) as i32;
    }
    pub fn offset_year_month(&self, offset: usize) -> (i32, u32) {
        let n = self.current_month as i64 - 1 + offset as i64;
        (
            self.current_year + n.div_euclid(12) as i32,
            n.rem_euclid(12) as u32 + 1,
        )
    }
    pub fn days(&self) -> Vec<Vec<NaiveDate>> {
        self.month_days().into_iter().flatten().collect()
    }
    /// Calendar weeks grouped by visible month. This preserves six-week
    /// months and is the preferred rendering API.
    pub fn month_days(&self) -> Vec<Vec<Vec<NaiveDate>>> {
        (0..self.number_of_months)
            .map(|n| {
                days_in_month(
                    self.current_year,
                    self.current_month as u32 + n as u32,
                    Weekday::Sun,
                )
            })
            .collect()
    }
    pub fn has_prev_year_page(&self) -> bool {
        self.year_page > 0
    }
    pub fn has_next_year_page(&self) -> bool {
        self.year_page < self.years.len() as i32 - 1
    }
    pub fn prev_year_page(&mut self) -> bool {
        if !self.has_prev_year_page() {
            false
        } else {
            self.year_page -= 1;
            true
        }
    }
    pub fn next_year_page(&mut self) -> bool {
        if !self.has_next_year_page() {
            false
        } else {
            self.year_page += 1;
            true
        }
    }
    pub fn prev_month(&mut self) {
        if self.current_month == 1 {
            self.current_year -= 1;
            self.current_month = 12;
        } else {
            self.current_month -= 1;
        }
    }
    pub fn next_month(&mut self) {
        if self.current_month == 12 {
            self.current_year += 1;
            self.current_month = 1;
        } else {
            self.current_month += 1;
        }
    }
    pub fn view(&self) -> CalendarView {
        self.view
    }
    pub fn set_view(&mut self, view: CalendarView) {
        self.view = view;
    }
    pub fn current_year(&self) -> i32 {
        self.current_year
    }
    pub fn current_month(&self) -> u8 {
        self.current_month
    }
    pub fn today(&self) -> NaiveDate {
        self.today
    }
    pub fn years_on_page(&self) -> &[i32] {
        self.years
            .get(self.year_page as usize)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
    pub fn select_month(&mut self, month: u8) {
        self.current_month = month;
        self.view = CalendarView::Day;
    }
    pub fn select_year(&mut self, year: i32) {
        self.current_year = year;
        self.view = CalendarView::Day;
    }

    /// Whether `date` is disabled by the disabled matcher.
    pub fn is_date_disabled(&self, date: &NaiveDate) -> bool {
        self.disabled_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.matched(date))
    }

    /// The day the keyboard acts on: the last day focused while its month is
    /// shown, else the selected day, today, or the first of the first month
    /// shown.
    pub fn focused_date(&self) -> NaiveDate {
        let shown = |date: &NaiveDate| self.is_month_shown(date);
        self.focused_date
            .filter(shown)
            .or(self.date.start().filter(shown))
            .or(Some(self.today).filter(shown))
            .unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(self.current_year, self.current_month as u32, 1)
                    .unwrap_or(self.today)
            })
    }

    /// Focus `date` for the keyboard, paging the months shown to it.
    pub fn set_focused_date(&mut self, date: NaiveDate) {
        self.focused_date = Some(date);
        self.show_month_of(date);
    }

    fn is_month_shown(&self, date: &NaiveDate) -> bool {
        let first = month_index(self.current_year, self.current_month as u32);
        let index = month_index(date.year(), date.month());
        (first..first + self.number_of_months as i64).contains(&index)
    }

    /// Pages the months shown as little as it takes to show `date`'s month.
    fn show_month_of(&mut self, date: NaiveDate) {
        let first = month_index(self.current_year, self.current_month as u32);
        let index = month_index(date.year(), date.month());
        let last = first + self.number_of_months.max(1) as i64 - 1;
        let first = if index < first {
            index
        } else if index > last {
            index - (last - first)
        } else {
            return;
        };
        self.current_year = first.div_euclid(12) as i32;
        self.current_month = first.rem_euclid(12) as u8 + 1;
    }

    /// Moves the focused day, skipping disabled days. Returns whether it
    /// moved.
    fn move_focus(&mut self, movement: FocusMove, first_day: Weekday) -> bool {
        let origin = self.focused_date();
        let Some((target, skip)) = movement.target(origin, first_day) else {
            return false;
        };
        match self.nearest_enabled(origin, target, skip) {
            Some(date) if date != origin => {
                self.set_focused_date(date);
                true
            }
            _ => false,
        }
    }

    fn nearest_enabled(
        &self,
        origin: NaiveDate,
        target: NaiveDate,
        skip: i64,
    ) -> Option<NaiveDate> {
        let enabled = |date: &NaiveDate| !self.is_date_disabled(date);
        if skip != 0 {
            let skipped = std::iter::successors(Some(target), |date| {
                date.checked_add_signed(chrono::Duration::days(skip))
            })
            .take((MAX_SKIPPED_DAYS / skip.abs()) as usize)
            .find(enabled);
            if skipped.is_some() {
                return skipped;
            }
        }
        // Nothing enabled ahead (past `max`): stop at the last enabled day
        // on the way back.
        let back = if target > origin { -1 } else { 1 };
        std::iter::successors(Some(target), |date| {
            date.checked_add_signed(chrono::Duration::days(back))
        })
        .take_while(|date| *date != origin)
        .find(enabled)
    }
}
impl EventEmitter<CalendarEvent> for CalendarState {}
impl Render for CalendarState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// The semantic kind of a calendar control rendered by [`Calendar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarItemKind {
    Previous,
    MonthToggle,
    YearToggle,
    Next,
    Weekday,
    Day,
    Month,
    Year,
}

/// State exposed to a calendar item slot. Applications may use it solely to
/// decorate the unstyled primitive; all interaction remains owned by base.
///
/// The fields are private and reached through the methods below, so that a new
/// one can be added without breaking the item slots.
#[derive(Clone, Copy, Debug)]
pub struct CalendarItemState {
    kind: CalendarItemKind,
    active: bool,
    in_range: bool,
    muted: bool,
    disabled: bool,
    today: bool,
    focused: bool,
}

impl CalendarItemState {
    /// Create a state for `kind`, with every flag off.
    pub fn new(kind: CalendarItemKind) -> Self {
        Self {
            kind,
            active: false,
            in_range: false,
            muted: false,
            disabled: false,
            today: false,
            focused: false,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Set whether the item is between the two ends of a range selection.
    pub fn in_range(mut self, in_range: bool) -> Self {
        self.in_range = in_range;
        self
    }

    /// Set whether the item is shown as secondary, e.g.: a weekday header or a
    /// day that belongs to the neighboring month.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn today(mut self, today: bool) -> Self {
        self.today = today;
        self
    }

    /// Set whether the item is the day the keyboard acts on, shown while the
    /// calendar has focus from the keyboard.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn kind(&self) -> CalendarItemKind {
        self.kind
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn is_in_range(&self) -> bool {
        self.in_range
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn is_today(&self) -> bool {
        self.today
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }
}

/// An unstyled, pre-wired calendar item passed to the item slot.
#[derive(IntoElement)]
pub struct CalendarItem {
    base: crate::ObservedElement<gpui::Stateful<gpui::Div>>,
    state: CalendarItemState,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl CalendarItem {
    fn new(id: impl Into<ElementId>, state: CalendarItemState) -> Self {
        Self {
            base: div().id(id.into()).test_support(),
            state,
            style: StyleRefinement::default(),
            children: vec![],
        }
    }
    // Keep the default visible label available to assistive technology even
    // when a presentation assigns a role that does not derive a name from children.
    fn with_label(self, label: SharedString) -> Self {
        self.aria_label(label.clone()).child(label)
    }

    pub fn item_state(&self) -> CalendarItemState {
        self.state
    }

    /// Remove the default label so a styled facade can provide custom content.
    pub fn clear_children(mut self) -> Self {
        self.children.clear();
        self
    }
}
impl ParentElement for CalendarItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
impl Styled for CalendarItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl InteractiveElement for CalendarItem {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }

    fn track_focus(mut self, handle: &gpui::FocusHandle) -> Self {
        self.base = self.base.track_focus(handle);
        self
    }
}
impl StatefulInteractiveElement for CalendarItem {}
impl RenderOnce for CalendarItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        // A weekday header is never interactive, so it is not "disabled".
        let disabled = self.state.is_disabled() && self.state.kind() != CalendarItemKind::Weekday;
        self.base
            .children(self.children)
            .refine_style(&self.style)
            .a11y_disabled(disabled)
    }
}

type ItemRenderer =
    Rc<dyn Fn(CalendarItem, CalendarItemState, &mut Window, &mut App) -> AnyElement>;
type Labeler = Rc<dyn Fn(CalendarItemKind, i32) -> SharedString>;

/// Complete unstyled calendar structure and behavior.
///
/// Base owns navigation, view switching, grids, disabled/selection state and
/// click handling. The UI crate only decorates the pre-wired item slot.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    state: Entity<CalendarState>,
    number_of_months: usize,
    first_day_of_week: Weekday,
    style: StyleRefinement,
    item: ItemRenderer,
    label: Labeler,
}

impl Calendar {
    pub fn new(id: impl Into<ElementId>, state: &Entity<CalendarState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            number_of_months: 1,
            first_day_of_week: Weekday::Sun,
            style: StyleRefinement::default(),
            item: Rc::new(|item, _, _, _| item.into_any_element()),
            label: Rc::new(|kind, value| match kind {
                CalendarItemKind::Previous => "‹".into(),
                CalendarItemKind::Next => "›".into(),
                CalendarItemKind::Weekday => value.to_string().into(),
                _ => value.to_string().into(),
            }),
        }
    }
    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count.max(1);
        self
    }
    pub fn first_day_of_week(mut self, day: Weekday) -> Self {
        self.first_day_of_week = day;
        self
    }
    pub fn item(
        mut self,
        render: impl Fn(CalendarItem, CalendarItemState, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.item = Rc::new(render);
        self
    }
    pub fn label(
        mut self,
        label: impl Fn(CalendarItemKind, i32) -> SharedString + 'static,
    ) -> Self {
        self.label = Rc::new(label);
        self
    }
}
impl Styled for Calendar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.number_of_months;
        self.state
            .update(cx, |s, cx| s.set_number_of_months(count, window, cx));
        let view = self.state.read(cx).view();
        let mut header = h_flex().items_center().justify_between().child({
            let st = CalendarItemState::new(CalendarItemKind::Previous).disabled(
                view.is_month() || (view.is_year() && !self.state.read(cx).has_prev_year_page()),
            );
            let mut item =
                CalendarItem::new("calendar-prev", st).with_label((self.label)(st.kind(), 0));
            if !st.is_disabled() {
                let entity = self.state.clone();
                item = item.on_click(move |_, _window, cx| {
                    entity.update(cx, |s, cx| {
                        if s.view().is_day() {
                            s.prev_month();
                        } else {
                            s.prev_year_page();
                        }
                        cx.notify();
                    })
                });
            }
            (self.item)(item, st, window, cx)
        });
        if count == 1 {
            let (month, year) = {
                let s = self.state.read(cx);
                (s.current_month() as i32, s.current_year())
            };
            for (kind, value, active) in [
                (CalendarItemKind::MonthToggle, month, view.is_month()),
                (CalendarItemKind::YearToggle, year, view.is_year()),
            ] {
                let st = CalendarItemState::new(kind).active(active);
                let entity = self.state.clone();
                let mut item = CalendarItem::new(format!("calendar-{kind:?}"), st)
                    .with_label((self.label)(kind, value));
                item = item.on_click(move |_, _, cx| {
                    entity.update(cx, |s, cx| {
                        s.set_view(
                            if s.view()
                                == match kind {
                                    CalendarItemKind::MonthToggle => CalendarView::Month,
                                    _ => CalendarView::Year,
                                }
                            {
                                CalendarView::Day
                            } else {
                                match kind {
                                    CalendarItemKind::MonthToggle => CalendarView::Month,
                                    _ => CalendarView::Year,
                                }
                            },
                        );
                        cx.notify();
                    })
                });
                header = header.child((self.item)(item, st, window, cx));
            }
        } else {
            for offset in 0..count {
                let (y, m) = self.state.read(cx).offset_year_month(offset);
                header = header.child(
                    div().text_sm().font_medium().child(
                        v_flex()
                            .items_center()
                            .child((self.label)(CalendarItemKind::MonthToggle, m as i32))
                            .child(y.to_string()),
                    ),
                );
            }
        }
        header = header.child({
            let st = CalendarItemState::new(CalendarItemKind::Next).disabled(
                view.is_month() || (view.is_year() && !self.state.read(cx).has_next_year_page()),
            );
            let mut item =
                CalendarItem::new("calendar-next", st).with_label((self.label)(st.kind(), 0));
            if !st.is_disabled() {
                let entity = self.state.clone();
                item = item.on_click(move |_, _, cx| {
                    entity.update(cx, |s, cx| {
                        if s.view().is_day() {
                            s.next_month()
                        } else {
                            s.next_year_page();
                        }
                        cx.notify();
                    })
                });
            }
            (self.item)(item, st, window, cx)
        });

        let mut body = match picker_grid_layout(view) {
            None => h_flex().justify_around(),
            Some((columns, horizontal_gap)) => {
                div().grid().grid_cols(columns).gap_x(px(horizontal_gap))
            }
        };
        let focus_handle = self.state.read(cx).focus_handle.clone();
        if view.is_day() {
            let focused_date = self.state.read(cx).focused_date();
            let focus_visible = focus_handle.is_focused(window) && window.last_input_was_keyboard();
            for offset in 0..count {
                let (year, month_number) = self.state.read(cx).offset_year_month(offset);
                let weeks = days_in_month(year, month_number, self.first_day_of_week);
                let mut month = v_flex()
                    .id(format!("calendar-grid-{offset}"))
                    .test_support()
                    .role(Role::Grid)
                    .aria_label(format!(
                        "{} {year}",
                        (self.label)(CalendarItemKind::MonthToggle, month_number as i32)
                    ));
                let mut header_row = h_flex()
                    .id(format!("weekdays-{offset}"))
                    .test_support()
                    .role(Role::Row);
                for weekday in 0..7 {
                    let st = CalendarItemState::new(CalendarItemKind::Weekday)
                        .muted(true)
                        .disabled(true);
                    let value =
                        (weekday + self.first_day_of_week.num_days_from_sunday() as i32) % 7;
                    let label = (self.label)(st.kind(), value);
                    let item = CalendarItem::new(format!("weekday-{offset}-{weekday}"), st)
                        .with_label(label)
                        .role(Role::ColumnHeader);
                    header_row = header_row.child((self.item)(item, st, window, cx));
                }
                month = month.child(header_row);
                for (week_index, week) in weeks.iter().enumerate() {
                    let mut week_row = h_flex()
                        .id(format!("calendar-week-{offset}-{week_index}"))
                        .test_support()
                        .role(Role::Row);
                    for date in week {
                        let date = *date;
                        // A day of a neighboring month also shows in its own
                        // month's grid, which holds the focus.
                        let focused = date == focused_date && date.month() == month_number;
                        let st = {
                            let s = self.state.read(cx);
                            let disabled = s.is_date_disabled(&date);
                            CalendarItemState::new(CalendarItemKind::Day)
                                .active(s.date().is_active(&date))
                                .in_range(s.date().is_in_range(&date))
                                .muted(date.month() != month_number || disabled)
                                .disabled(disabled)
                                .today(date == s.today())
                                .focused(focused && focus_visible)
                        };
                        let mut item =
                            CalendarItem::new(format!("calendar-{date}-{offset}-{week_index}"), st)
                                .with_label((self.label)(st.kind(), date.day() as i32))
                                .aria_label(date.to_string())
                                .role(Role::GridCell)
                                .aria_selected(st.is_active() || st.is_in_range());
                        if focused {
                            item = item.aria_active_descendant();
                        }
                        if !st.is_disabled() {
                            let entity = self.state.clone();
                            item = item.on_click(move |_, _, cx| {
                                entity.update(cx, |s, cx| {
                                    s.activate_date(date, cx);
                                })
                            });
                        }
                        week_row = week_row.child((self.item)(item, st, window, cx));
                    }
                    month = month.child(week_row);
                }
                body = body.child(month);
            }
        } else if view.is_month() {
            let current = self.state.read(cx).current_month();
            for month in 1..=12u8 {
                let st = CalendarItemState::new(CalendarItemKind::Month).active(month == current);
                let entity = self.state.clone();
                let item = CalendarItem::new(format!("calendar-month-{month}"), st)
                    .with_label((self.label)(st.kind(), month as i32))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |s, cx| {
                            s.select_month(month);
                            cx.notify();
                        })
                    });
                body = body.child((self.item)(item, st, window, cx));
            }
        } else {
            let current = self.state.read(cx).current_year();
            let years = self.state.read(cx).years_on_page().to_vec();
            for year in years {
                let st = CalendarItemState::new(CalendarItemKind::Year).active(year == current);
                let entity = self.state.clone();
                let item = CalendarItem::new(format!("calendar-year-{year}"), st)
                    .with_label((self.label)(st.kind(), year))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |s, cx| {
                            s.select_year(year);
                            cx.notify();
                        })
                    });
                body = body.child((self.item)(item, st, window, cx));
            }
        }
        let state = &self.state;
        let first_day = self.first_day_of_week;
        v_flex()
            .id(self.id)
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(on_move::<PreviousDay>(
                state,
                FocusMove::Days(-1),
                first_day,
            ))
            .on_action(on_move::<NextDay>(state, FocusMove::Days(1), first_day))
            .on_action(on_move::<PreviousWeek>(
                state,
                FocusMove::Days(-7),
                first_day,
            ))
            .on_action(on_move::<NextWeek>(state, FocusMove::Days(7), first_day))
            .on_action(on_move::<PreviousMonth>(
                state,
                FocusMove::Months(-1),
                first_day,
            ))
            .on_action(on_move::<NextMonth>(state, FocusMove::Months(1), first_day))
            .on_action(on_move::<PreviousYear>(
                state,
                FocusMove::Months(-12),
                first_day,
            ))
            .on_action(on_move::<NextYear>(state, FocusMove::Months(12), first_day))
            .on_action(on_move::<WeekStart>(state, FocusMove::WeekStart, first_day))
            .on_action(on_move::<WeekEnd>(state, FocusMove::WeekEnd, first_day))
            .on_action({
                let entity = self.state.clone();
                move |_: &Confirm, _, cx| {
                    entity.update(cx, |s, cx| {
                        let date = s.focused_date();
                        if !s.view().is_day() {
                            cx.propagate();
                        } else if !s.is_date_disabled(&date) {
                            s.activate_date(date, cx);
                        }
                    })
                }
            })
            .child(header)
            .child(body)
            .refine_style(&self.style)
    }
}

/// The handler of action `A`, moving the focused day of `state` in the day
/// view.
fn on_move<A: Action>(
    state: &Entity<CalendarState>,
    movement: FocusMove,
    first_day: Weekday,
) -> impl Fn(&A, &mut Window, &mut App) + 'static {
    let state = state.clone();
    move |_, _, cx| {
        state.update(cx, |state, cx| {
            if !state.view().is_day() {
                cx.propagate();
            } else if state.move_focus(movement, first_day) {
                cx.notify();
            }
        })
    }
}

fn days_in_month(year: i32, month: u32, first_day: Weekday) -> Vec<Vec<NaiveDate>> {
    let total = year as i64 * 12 + month as i64 - 1;
    let year = total.div_euclid(12) as i32;
    let month = total.rem_euclid(12) as u32 + 1;
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    let offset =
        (first.weekday().num_days_from_sunday() + 7 - first_day.num_days_from_sunday()) % 7;
    let start = first - chrono::Duration::days(offset as i64);
    let count = ((next - start).num_days() as usize).div_ceil(7) * 7;
    (0..count)
        .map(|n| start + chrono::Duration::days(n as i64))
        .collect::<Vec<_>>()
        .chunks(7)
        .map(<[_]>::to_vec)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext as _, Context, Entity, IntoElement, Render, Subscription, Window};

    use super::*;

    #[gpui::test]
    fn calendar_items_expose_names_in_every_view(cx: &mut gpui::TestAppContext) {
        use gpui::{Element as _, accesskit};
        struct Names {
            calendar: Entity<CalendarState>,
            labels: Rc<RefCell<Vec<(CalendarItemKind, String)>>>,
        }
        impl Render for Names {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let labels = self.labels.clone();
                labels.borrow_mut().clear();
                Calendar::new("named-calendar", &self.calendar).item(move |item, state, _, _| {
                    let mut node = accesskit::Node::new(accesskit::Role::Button);
                    item.base.write_a11y_info(&mut node);
                    labels
                        .borrow_mut()
                        .push((state.kind(), node.label().unwrap_or("").to_string()));
                    item.into_any_element()
                })
            }
        }
        let (view, cx) = cx.add_window_view(|window, cx| Names {
            calendar: cx.new(|cx| CalendarState::new(window, cx)),
            labels: Rc::new(RefCell::new(Vec::new())),
        });
        for mode in [CalendarView::Day, CalendarView::Month, CalendarView::Year] {
            cx.update(|window, cx| {
                view.read(cx).calendar.clone().update(cx, |state, cx| {
                    state.apply_date(Date::Single(Some(
                        NaiveDate::from_ymd_opt(2026, 9, 7).unwrap(),
                    )));
                    state.set_view(mode);
                    cx.notify();
                });
                window.draw(cx).clear(cx);
                let labels = view.read(cx).labels.borrow();
                assert!(!labels.is_empty());
                for (kind, label) in labels.iter() {
                    assert!(!label.is_empty(), "missing name for {kind:?}");
                }
                if mode.is_day() {
                    assert!(labels.iter().any(
                        |(kind, label)| *kind == CalendarItemKind::Day && label == "2026-09-07"
                    ));
                }
            });
        }
    }

    struct EventHarness {
        calendar: Entity<CalendarState>,
        events: Rc<RefCell<Vec<Date>>>,
        _subscription: Option<Subscription>,
    }
    impl EventHarness {
        fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
            let calendar = cx.new(|cx| CalendarState::new(window, cx));
            let events = Rc::new(RefCell::new(Vec::new()));
            let mut this = Self {
                calendar: calendar.clone(),
                events: events.clone(),
                _subscription: None,
            };
            this._subscription = Some(cx.subscribe(&calendar, move |_, _, event, _| {
                let CalendarEvent::Selected(date) = event;
                events.borrow_mut().push(*date);
            }));
            this
        }
    }
    impl Render for EventHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Empty
        }
    }
    fn state(cx: &mut gpui::TestAppContext, date: Date) -> gpui::Entity<CalendarState> {
        let (state, _) = cx.add_window_view(CalendarState::new);
        state.update(cx, |state, _| {
            state.date = date;
        });
        state
    }
    #[gpui::test]
    fn range_selection_restarts_and_completes(cx: &mut gpui::TestAppContext) {
        let s = state(cx, Date::Range(None, None));
        let a = NaiveDate::from_ymd_opt(2025, 2, 10).unwrap();
        let b = NaiveDate::from_ymd_opt(2025, 2, 12).unwrap();
        s.update(cx, |s, _| {
            assert!(!s.select_date(a));
            assert!(s.select_date(b));
        });
        assert_eq!(
            s.read_with(cx, |s, _| s.date()),
            Date::Range(Some(a), Some(b))
        );
        s.update(cx, |s, _| assert!(!s.select_date(a)));
        assert_eq!(s.read_with(cx, |s, _| s.date()), Date::Range(Some(a), None));
    }
    #[gpui::test]
    fn disabled_date_is_rejected(cx: &mut gpui::TestAppContext) {
        let s = state(cx, Date::Single(None));
        s.update(cx, |s, _| {
            s.disabled_matcher = Some(Rc::new(Matcher::range(
                Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()),
            )))
        });
        s.update(cx, |s, _| {
            assert!(!s.select_date(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()))
        });
    }
    #[gpui::test]
    fn month_navigation_crosses_year(cx: &mut gpui::TestAppContext) {
        let s = state(
            cx,
            Date::Single(Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())),
        );
        s.update(cx, |s, _| {
            s.apply_date(Date::Single(Some(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            )));
            s.prev_month();
        });
        assert_eq!(
            s.read_with(cx, |s, _| (s.current_year(), s.current_month())),
            (2024, 12)
        );
        s.update(cx, |s, _| s.next_month());
        assert_eq!(
            s.read_with(cx, |s, _| (s.current_year(), s.current_month())),
            (2025, 1)
        );
    }

    #[gpui::test]
    fn six_week_month_is_not_truncated(cx: &mut gpui::TestAppContext) {
        let s = state(
            cx,
            Date::Single(Some(NaiveDate::from_ymd_opt(2025, 8, 1).unwrap())),
        );
        s.update(cx, |s, _| {
            s.apply_date(Date::Single(Some(
                NaiveDate::from_ymd_opt(2025, 8, 1).unwrap(),
            )));
            assert_eq!(s.month_days().len(), 1);
            assert_eq!(s.month_days()[0].len(), 6);
            assert_eq!(s.days().len(), 6);
            assert_eq!(s.month_days()[0][5][0].day(), 31);
        });
    }

    #[gpui::test]
    fn day_month_and_year_views_have_complete_transitions(cx: &mut gpui::TestAppContext) {
        let s = state(cx, Date::Single(None));
        s.update(cx, |s, _| {
            assert_eq!(s.view(), CalendarView::Day);
            s.set_view(CalendarView::Month);
            s.select_month(11);
            assert_eq!((s.view(), s.current_month()), (CalendarView::Day, 11));
            s.set_view(CalendarView::Year);
            s.select_year(2032);
            assert_eq!((s.view(), s.current_year()), (CalendarView::Day, 2032));
        });
    }

    #[test]
    fn picker_views_use_stable_grid_layouts() {
        assert_eq!(picker_grid_layout(CalendarView::Month), Some((3, 4.)));
        assert_eq!(picker_grid_layout(CalendarView::Year), Some((5, 4.)));
        assert_eq!(picker_grid_layout(CalendarView::Day), None);
    }

    #[gpui::test]
    fn year_page_navigation_respects_both_bounds(cx: &mut gpui::TestAppContext) {
        let s = state(cx, Date::Single(None));
        s.update(cx, |s, _| {
            s.apply_year_range((2000, 2041));
            while s.prev_year_page() {}
            assert!(!s.has_prev_year_page());
            assert!(!s.prev_year_page());
            assert!(s.next_year_page());
            while s.next_year_page() {}
            assert!(!s.has_next_year_page());
            assert!(!s.next_year_page());
        });
    }

    #[gpui::test]
    fn activation_emits_only_for_complete_enabled_values(cx: &mut gpui::TestAppContext) {
        let (harness, _) = cx.add_window_view(EventHarness::new);
        let s = harness.read_with(cx, |h, _| h.calendar.clone());
        s.update(cx, |s, _| s.date = Date::Range(None, None));
        let start = NaiveDate::from_ymd_opt(2025, 4, 4).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 8).unwrap();
        s.update(cx, |s, cx| {
            assert!(!s.activate_date(start, cx));
            assert!(s.activate_date(end, cx));
            assert_eq!(s.date(), Date::Range(Some(start), Some(end)));
            s.set_disabled_matcher_shared(Some(Rc::new(Matcher::custom(move |d| *d == start))));
            assert!(!s.activate_date(start, cx));
            assert_eq!(s.date(), Date::Range(Some(start), Some(end)));
        });
        assert_eq!(
            harness.read_with(cx, |h, _| h.events.borrow().clone()),
            vec![Date::Range(Some(start), Some(end))]
        );

        s.update(cx, |s, cx| {
            s.date = Date::Single(None);
            assert!(s.activate_date(end, cx));
            assert_eq!(s.date(), Date::Single(Some(end)));
        });
        assert_eq!(
            harness.read_with(cx, |h, _| h.events.borrow().clone()),
            vec![Date::Range(Some(start), Some(end)), Date::Single(Some(end))]
        );
    }

    fn ymd(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// A calendar driven through real keystrokes. The item slot records the
    /// focused day and the selected days of the last frame.
    struct KeyboardHarness {
        calendar: Entity<CalendarState>,
        months: usize,
        first_day: Weekday,
        focused: Rc<RefCell<Vec<String>>>,
        events: Rc<RefCell<Vec<Date>>>,
        _subscription: Subscription,
    }
    impl Render for KeyboardHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            use gpui::{Element as _, accesskit};
            let focused = self.focused.clone();
            focused.borrow_mut().clear();
            Calendar::new("keyboard-calendar", &self.calendar)
                .number_of_months(self.months)
                .first_day_of_week(self.first_day)
                .item(move |item, state, _, _| {
                    if state.is_focused() {
                        let mut node = accesskit::Node::new(accesskit::Role::GridCell);
                        item.base.write_a11y_info(&mut node);
                        focused
                            .borrow_mut()
                            .push(node.label().unwrap_or("").to_string());
                    }
                    item.into_any_element()
                })
        }
    }

    fn keyboard(
        cx: &mut gpui::TestAppContext,
        date: Date,
        months: usize,
        first_day: Weekday,
    ) -> (Entity<KeyboardHarness>, &mut gpui::VisualTestContext) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let calendar = cx.new(|cx| CalendarState::new(window, cx));
            calendar.update(cx, |state, _| state.apply_date(date));
            let events = Rc::new(RefCell::new(Vec::new()));
            let _subscription = cx.subscribe(&calendar, {
                let events = events.clone();
                move |_, _, CalendarEvent::Selected(date), _| events.borrow_mut().push(*date)
            });
            KeyboardHarness {
                calendar,
                months,
                first_day,
                focused: Rc::default(),
                events,
                _subscription,
            }
        });
        cx.update(|window, cx| {
            let focus = view.read(cx).calendar.read(cx).focus_handle.clone();
            focus.focus(window, cx);
        });
        cx.run_until_parked();
        (view, cx)
    }

    fn focused(view: &Entity<KeyboardHarness>, cx: &mut gpui::VisualTestContext) -> NaiveDate {
        cx.run_until_parked();
        view.read_with(cx, |view, cx| view.calendar.read(cx).focused_date())
    }

    fn shown(view: &Entity<KeyboardHarness>, cx: &mut gpui::VisualTestContext) -> (i32, u8) {
        view.read_with(cx, |view, cx| {
            let state = view.calendar.read(cx);
            (state.current_year(), state.current_month())
        })
    }

    #[gpui::test]
    fn keys_move_the_focused_day_and_the_months_follow(cx: &mut gpui::TestAppContext) {
        let (view, cx) = keyboard(cx, Date::Single(Some(ymd(2025, 1, 31))), 1, Weekday::Mon);
        assert_eq!(focused(&view, cx), ymd(2025, 1, 31));
        cx.simulate_keystrokes("right");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 1));
        assert_eq!(shown(&view, cx), (2025, 2));
        cx.simulate_keystrokes("left up");
        assert_eq!(focused(&view, cx), ymd(2025, 1, 24));
        assert_eq!(shown(&view, cx), (2025, 1));
        cx.simulate_keystrokes("down down");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 7));
        // A month on clamps to its last day; Shift moves by a year.
        cx.simulate_keystrokes("pageup");
        assert_eq!(focused(&view, cx), ymd(2025, 1, 7));
        cx.simulate_keystrokes("shift-pagedown");
        assert_eq!(focused(&view, cx), ymd(2026, 1, 7));
        assert_eq!(shown(&view, cx), (2026, 1));
        cx.simulate_keystrokes("shift-pageup pagedown");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 7));
        // January 31 in a week that starts on Monday: Home and End.
        view.update(cx, |view, cx| {
            view.calendar
                .update(cx, |state, _| state.set_focused_date(ymd(2025, 1, 31)))
        });
        cx.simulate_keystrokes("pagedown");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 28));
        cx.simulate_keystrokes("home");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 24));
        cx.simulate_keystrokes("end");
        assert_eq!(focused(&view, cx), ymd(2025, 3, 2));
        assert!(view.read_with(cx, |view, _| view.events.borrow().is_empty()));
    }

    #[gpui::test]
    fn enter_and_space_select_the_focused_day(cx: &mut gpui::TestAppContext) {
        let (view, cx) = keyboard(cx, Date::Range(None, None), 1, Weekday::Sun);
        view.update(cx, |view, cx| {
            view.calendar
                .update(cx, |state, _| state.set_focused_date(ymd(2025, 4, 4)))
        });
        cx.simulate_keystrokes("enter right right space");
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.events.borrow().clone()),
            vec![Date::Range(Some(ymd(2025, 4, 4)), Some(ymd(2025, 4, 6)))]
        );
        // The focus stays on the day that completed the range.
        assert_eq!(focused(&view, cx), ymd(2025, 4, 6));
    }

    #[gpui::test]
    fn keys_skip_disabled_days_and_stop_at_the_bounds(cx: &mut gpui::TestAppContext) {
        let (view, cx) = keyboard(cx, Date::Single(Some(ymd(2025, 2, 7))), 1, Weekday::Sun);
        // Weekends are disabled: Friday's next day is Monday.
        view.update(cx, |view, cx| {
            view.calendar.update(cx, |state, _| {
                state.set_disabled_matcher_shared(Some(Rc::new(Matcher::DayOfWeek(vec![0, 6]))))
            })
        });
        cx.simulate_keystrokes("right");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 10));
        cx.simulate_keystrokes("left");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 7));
        // Home lands on Sunday, disabled: the nearest enabled day back.
        cx.simulate_keystrokes("home");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 3));

        // `min` and `max`: keys stop at them, a page lands on them.
        view.update(cx, |view, cx| {
            view.calendar.update(cx, |state, _| {
                state.set_disabled_matcher_shared(Some(Rc::new(Matcher::interval(
                    Some(ymd(2025, 2, 1)),
                    Some(ymd(2025, 2, 20)),
                ))))
            })
        });
        cx.simulate_keystrokes("pagedown");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 20));
        cx.simulate_keystrokes("right down");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 20));
        cx.simulate_keystrokes("shift-pageup");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 1));
        cx.simulate_keystrokes("left up enter");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 1));
        assert_eq!(
            view.read_with(cx, |view, _| view.events.borrow().clone()),
            vec![Date::Single(Some(ymd(2025, 2, 1)))]
        );
    }

    #[gpui::test]
    fn the_months_shown_page_only_as_far_as_the_focus(cx: &mut gpui::TestAppContext) {
        let (view, cx) = keyboard(cx, Date::Single(Some(ymd(2025, 1, 20))), 2, Weekday::Sun);
        cx.simulate_keystrokes("pagedown");
        assert_eq!(focused(&view, cx), ymd(2025, 2, 20));
        assert_eq!(shown(&view, cx), (2025, 1));
        cx.simulate_keystrokes("pagedown");
        assert_eq!(shown(&view, cx), (2025, 2));
        // Selecting a day in the second month does not page.
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!(shown(&view, cx), (2025, 2));
        assert_eq!(
            view.read_with(cx, |view, _| view.events.borrow().clone()),
            vec![Date::Single(Some(ymd(2025, 3, 20)))]
        );
    }

    #[gpui::test]
    fn the_focused_day_shows_only_with_keyboard_focus(cx: &mut gpui::TestAppContext) {
        let (view, cx) = keyboard(cx, Date::Single(Some(ymd(2025, 3, 31))), 2, Weekday::Sun);
        let shown = |view: &Entity<KeyboardHarness>, cx: &mut gpui::VisualTestContext| {
            cx.run_until_parked();
            view.read_with(cx, |view, _| view.focused.borrow().clone())
        };
        // Focus from a click or the program shows no focused day.
        assert!(shown(&view, cx).is_empty());
        // March 31 also shows, muted, in April's first week; only March's
        // grid holds the focus.
        cx.simulate_keystrokes("left right");
        assert_eq!(shown(&view, cx), vec!["2025-03-31".to_string()]);
        cx.update(|window, cx| window.blur(cx));
        assert!(shown(&view, cx).is_empty());
    }

    #[gpui::test]
    fn days_form_a_grid_for_assistive_technology(cx: &mut gpui::TestAppContext) {
        let (view, cx) = keyboard(cx, Date::Single(Some(ymd(2025, 2, 10))), 1, Weekday::Sun);
        view.update(cx, |view, cx| {
            view.calendar.update(cx, |state, _| {
                state.set_disabled_matcher_shared(Some(Rc::new(Matcher::DayOfWeek(vec![0]))))
            })
        });
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let find = |id: &str| {
                crate::test_support::find(window, &[], &ElementId::Name(id.to_owned().into()))
                    .unwrap()
            };
            assert_eq!(find("calendar-grid-0").role(), Some(Role::Grid));
            assert_eq!(find("calendar-week-0-2").role(), Some(Role::Row));
            assert_eq!(find("weekday-0-0").role(), Some(Role::ColumnHeader));
            let selected = find("calendar-2025-02-10-0-2");
            assert_eq!(selected.role(), Some(Role::GridCell));
            assert_eq!(selected.selected(), Some(true));
            assert_eq!(find("calendar-2025-02-11-0-2").selected(), Some(false));
            assert_eq!(find("calendar-2025-02-09-0-2").disabled(), Some(true));
            assert_ne!(find("calendar-2025-02-11-0-2").disabled(), Some(true));
        });
    }
}
