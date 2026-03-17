use std::time::Duration;

use chrono::{Local, NaiveDate, NaiveTime, Timelike};
use gpui::{
    App, ClickEvent, Context, Div, ElementId, Empty, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Render, RenderOnce, SharedString, Stateful, StyleRefinement, Styled, Window,
    prelude::FluentBuilder as _, px,
};
use rust_i18n::t;

use crate::{
    ActiveTheme, Sizable, Size, StyledExt as _,
    button::ButtonVariants as _,
    h_flex,
    scroll::ScrollableElement as _,
    v_flex,
};

/// A time slot interval for the schedule view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotInterval(Duration);

impl SlotInterval {
    pub fn minutes(minutes: u32) -> Self {
        assert!(
            minutes > 0 && minutes <= 1440,
            "interval must be 1..=1440 minutes"
        );
        assert!(
            1440 % minutes == 0,
            "1440 (minutes/day) must be divisible by interval"
        );
        Self(Duration::from_secs(minutes as u64 * 60))
    }

    pub fn hours(hours: u32) -> Self {
        Self::minutes(
            hours
                .checked_mul(60)
                .expect("hours overflow when converting to minutes"),
        )
    }

    fn as_minutes(&self) -> u32 {
        (self.0.as_secs() / 60) as u32
    }

    pub fn slots_per_day(&self) -> u32 {
        1440 / self.as_minutes()
    }
}

impl Default for SlotInterval {
    fn default() -> Self {
        Self::hours(1)
    }
}

/// Identifies a single slot in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotPosition {
    pub date: NaiveDate,
    pub time: NaiveTime,
}

impl SlotPosition {
    fn cmp_key(&self) -> (NaiveDate, NaiveTime) {
        (self.date, self.time)
    }
}

impl PartialOrd for SlotPosition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SlotPosition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_key().cmp(&other.cmp_key())
    }
}

/// A selected time range, potentially spanning multiple days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// Start date of the range.
    pub start_date: NaiveDate,
    /// Inclusive start time on `start_date`.
    pub start: NaiveTime,
    /// End date of the range (may differ from `start_date`).
    pub end_date: NaiveDate,
    /// Exclusive end time on `end_date` (one interval past the last selected slot).
    pub end: NaiveTime,
}

/// Events emitted by the schedule view.
pub enum ScheduleEvent {
    /// A single time slot was clicked (no drag).
    SlotClicked(SlotPosition),
    /// A time range was selected via drag.
    RangeSelected(TimeRange),
}

/// State for the schedule view.
pub struct ScheduleViewState {
    focus_handle: FocusHandle,
    start_date: NaiveDate,
    num_days: usize,
    interval: SlotInterval,
    day_start_hour: u32,
    day_end_hour: u32,
    /// Active drag: anchor slot and current extent slot.
    drag: Option<(SlotPosition, SlotPosition)>,
}

impl ScheduleViewState {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let today = Local::now().naive_local().date();
        Self {
            focus_handle: cx.focus_handle(),
            start_date: today,
            num_days: 1,
            interval: SlotInterval::default(),
            day_start_hour: 0,
            day_end_hour: 24,
            drag: None,
        }
    }

    pub fn interval(mut self, interval: SlotInterval) -> Self {
        self.interval = interval;
        self
    }

    pub fn num_days(mut self, num_days: usize) -> Self {
        assert!(num_days >= 1);
        self.num_days = num_days;
        self
    }

    /// Restrict visible hours, e.g. `(8, 18)` for 08:00–18:00.
    pub fn visible_hours(mut self, start: u32, end: u32) -> Self {
        assert!(start < end && end <= 24);
        self.day_start_hour = start;
        self.day_end_hour = end;
        self
    }

    pub fn set_start_date(
        &mut self,
        date: NaiveDate,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_date = date;
        cx.notify();
    }

    pub fn set_interval(
        &mut self,
        interval: SlotInterval,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interval = interval;
        cx.notify();
    }

    pub fn set_num_days(
        &mut self,
        num_days: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        assert!(num_days >= 1);
        self.num_days = num_days;
        cx.notify();
    }

    pub fn start_date(&self) -> NaiveDate {
        self.start_date
    }

    pub fn dates(&self) -> Vec<NaiveDate> {
        (0..self.num_days as i64)
            .map(|offset| self.start_date + chrono::Duration::days(offset))
            .collect()
    }

    fn time_slots(&self) -> Vec<NaiveTime> {
        let start_minutes = self.day_start_hour * 60;
        let end_minutes = self.day_end_hour * 60;
        let step = self.interval.as_minutes();

        (0..)
            .map(|i| start_minutes + i * step)
            .take_while(|&m| m < end_minutes)
            .map(|m| NaiveTime::from_hms_opt(m / 60, m % 60, 0).unwrap())
            .collect()
    }

    /// Returns the normalized selection range during a drag,
    /// expanded to cover full slot intervals. May span multiple days.
    fn drag_range(&self) -> Option<TimeRange> {
        let (anchor, extent) = self.drag?;
        let (lo, hi) = if anchor <= extent {
            (anchor, extent)
        } else {
            (extent, anchor)
        };
        let end_pos = advance_slot(hi, self.interval.as_minutes());
        Some(TimeRange {
            start_date: lo.date,
            start: lo.time,
            end_date: end_pos.date,
            end: end_pos.time,
        })
    }

    fn is_slot_in_drag_range(&self, date: NaiveDate, time: NaiveTime) -> bool {
        if let Some(range) = self.drag_range() {
            let pos = SlotPosition { date, time };
            let start = SlotPosition {
                date: range.start_date,
                time: range.start,
            };
            let end = SlotPosition {
                date: range.end_date,
                time: range.end,
            };
            pos >= start && pos < end
        } else {
            false
        }
    }

    fn begin_drag(&mut self, pos: SlotPosition, cx: &mut Context<Self>) {
        self.drag = Some((pos, pos));
        cx.notify();
    }

    fn update_drag(&mut self, pos: SlotPosition, cx: &mut Context<Self>) {
        if let Some((_anchor, ref mut extent)) = self.drag {
            if *extent != pos {
                *extent = pos;
                cx.notify();
            }
        }
    }

    fn finish_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(range) = self.drag_range() {
            let (anchor, extent) = self.drag.unwrap();
            if anchor == extent {
                cx.emit(ScheduleEvent::SlotClicked(anchor));
            } else {
                cx.emit(ScheduleEvent::RangeSelected(range));
            }
        }
        self.drag = None;
        cx.notify();
    }

    fn navigate(&mut self, delta_days: i64, _: &mut Window, cx: &mut Context<Self>) {
        self.start_date += chrono::Duration::days(delta_days);
        cx.notify();
    }

    fn prev(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(-(self.num_days as i64), window, cx);
    }

    fn next(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(self.num_days as i64, window, cx);
    }

    fn today(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.start_date = Local::now().naive_local().date();
        cx.notify();
    }
}

/// Advance a slot position by one interval. Wraps across midnight correctly.
fn advance_slot(pos: SlotPosition, interval_minutes: u32) -> SlotPosition {
    let total = pos.time.hour() * 60 + pos.time.minute() + interval_minutes;
    let extra_days = (total / 1440) as i64;
    let remaining = total % 1440;
    SlotPosition {
        date: pos.date + chrono::Duration::days(extra_days),
        time: NaiveTime::from_hms_opt(remaining / 60, remaining % 60, 0).unwrap(),
    }
}

impl EventEmitter<ScheduleEvent> for ScheduleViewState {}

impl Render for ScheduleViewState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

// -- View element --

#[derive(IntoElement)]
pub struct ScheduleView {
    id: ElementId,
    size: Size,
    state: Entity<ScheduleViewState>,
    style: StyleRefinement,
}

impl ScheduleView {
    pub fn new(state: &Entity<ScheduleViewState>) -> Self {
        Self {
            id: ("schedule-view", state.entity_id()).into(),
            size: Size::default(),
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }

    fn format_time(time: &NaiveTime) -> SharedString {
        format!("{:02}:{:02}", time.hour(), time.minute()).into()
    }

    fn slot_height(&self) -> gpui::Pixels {
        match self.size {
            Size::Small => px(28.),
            Size::Large => px(48.),
            _ => px(36.),
        }
    }

    fn render_header(&self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon_size = match self.size {
            Size::Small => Size::Small,
            _ => Size::Medium,
        };

        h_flex()
            .gap_1()
            .items_center()
            .pb_1()
            .child(
                crate::button::Button::new("prev")
                    .icon(crate::IconName::ArrowLeft)
                    .ghost()
                    .tab_stop(false)
                    .with_size(icon_size)
                    .on_click(window.listener_for(&self.state, ScheduleViewState::prev)),
            )
            .child(
                crate::button::Button::new("today")
                    .ghost()
                    .tab_stop(false)
                    .label(t!("Calendar.today"))
                    .compact()
                    .with_size(icon_size)
                    .on_click(window.listener_for(&self.state, ScheduleViewState::today)),
            )
            .child(
                crate::button::Button::new("next")
                    .icon(crate::IconName::ArrowRight)
                    .ghost()
                    .tab_stop(false)
                    .with_size(icon_size)
                    .on_click(window.listener_for(&self.state, ScheduleViewState::next)),
            )
    }

    fn render_day_headers(&self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let dates = state.dates();
        let today = Local::now().naive_local().date();

        h_flex()
            .child(h_flex().w(px(52.)).flex_shrink_0())
            .children(dates.iter().map(|date| {
                let is_today = *date == today;
                let weekday: SharedString = date.format("%a").to_string().into();
                let day_num: SharedString = date.format("%d").to_string().into();

                h_flex()
                    .flex_1()
                    .border_l_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_center()
                    .py_1()
                    .gap_1()
                    .text_sm()
                    .child(
                        h_flex()
                            .text_color(cx.theme().muted_foreground)
                            .child(weekday),
                    )
                    .child(
                        h_flex()
                            .when(is_today, |this| {
                                this.text_color(cx.theme().primary_foreground)
                                    .bg(cx.theme().primary)
                                    .rounded(cx.theme().radius)
                                    .px_1()
                            })
                            .child(day_num),
                    )
            }))
    }

    fn render_time_label(&self, time: &NaiveTime, _window: &mut Window, cx: &mut App) -> Div {
        h_flex()
            .h(self.slot_height())
            .w(px(52.))
            .flex_shrink_0()
            .justify_end()
            .pr_2()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(Self::format_time(time))
    }

    fn render_slot(
        &self,
        date: NaiveDate,
        time: NaiveTime,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let state = self.state.read(cx);
        let in_drag = state.is_slot_in_drag_range(date, time);
        let slot_id: SharedString =
            format!("slot_{}_{}", date.format("%Y%m%d"), time.format("%H%M")).into();

        let pos = SlotPosition { date, time };

        h_flex()
            .id(slot_id)
            .h(self.slot_height())
            .flex_1()
            .border_b_1()
            .border_l_1()
            .border_color(cx.theme().border)
            .when(in_drag, |this| {
                this.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .when(!in_drag, |this| {
                this.hover(|this| this.bg(cx.theme().accent.opacity(0.5)))
            })
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(
                    &self.state,
                    move |view, _ev: &MouseDownEvent, _window, cx| {
                        view.begin_drag(pos, cx);
                    },
                ),
            )
            .on_mouse_move(window.listener_for(
                &self.state,
                move |view, _ev: &MouseMoveEvent, _window, cx| {
                    if view.drag.is_some() {
                        view.update_drag(pos, cx);
                    }
                },
            ))
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(
                    &self.state,
                    move |view, _ev: &MouseUpEvent, _window, cx| {
                        if view.drag.is_some() {
                            view.update_drag(pos, cx);
                            view.finish_drag(cx);
                        }
                    },
                ),
            )
    }

    fn render_grid(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let dates = state.dates();
        let time_slots = state.time_slots();

        v_flex().gap_0().children(time_slots.iter().map(|time| {
            h_flex()
                .child(self.render_time_label(time, window, cx))
                .children(
                    dates
                        .iter()
                        .map(|date| self.render_slot(*date, *time, window, cx)),
                )
        }))
    }
}

impl Sizable for ScheduleView {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for ScheduleView {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ScheduleView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .id(self.id.clone())
            .track_focus(&self.state.read(cx).focus_handle)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .overflow_hidden()
            .refine_style(&self.style)
            .map(|this| match self.size {
                Size::Small => this.text_sm(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .on_mouse_up_out(
                MouseButton::Left,
                window.listener_for(
                    &self.state,
                    |view, _ev: &MouseUpEvent, _window, cx| {
                        if view.drag.is_some() {
                            view.finish_drag(cx);
                        }
                    },
                ),
            )
            .child(v_flex().p_2().child(self.render_header(window, cx)))
            .child(self.render_day_headers(window, cx))
            .child(
                v_flex()
                    .overflow_y_scrollbar()
                    .child(self.render_grid(window, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext as _; // for .new(), .open_window() etc.

    #[test]
    fn test_slot_interval() {
        let interval = SlotInterval::hours(1);
        assert_eq!(interval.as_minutes(), 60);
        assert_eq!(interval.slots_per_day(), 24);

        let interval = SlotInterval::minutes(30);
        assert_eq!(interval.as_minutes(), 30);
        assert_eq!(interval.slots_per_day(), 48);

        let interval = SlotInterval::minutes(15);
        assert_eq!(interval.as_minutes(), 15);
        assert_eq!(interval.slots_per_day(), 96);
    }

    #[test]
    #[should_panic]
    fn test_slot_interval_zero() {
        SlotInterval::minutes(0);
    }

    #[test]
    #[should_panic]
    fn test_slot_interval_not_divisible() {
        SlotInterval::minutes(7);
    }

    #[test]
    fn test_advance_slot() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 17).unwrap();
        let pos = SlotPosition {
            date,
            time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        };
        let next = advance_slot(pos, 30);
        assert_eq!(next.date, date);
        assert_eq!(next.time, NaiveTime::from_hms_opt(10, 30, 0).unwrap());

        let next = advance_slot(pos, 60);
        assert_eq!(next.date, date);
        assert_eq!(next.time, NaiveTime::from_hms_opt(11, 0, 0).unwrap());
    }

    #[test]
    fn test_advance_slot_crosses_midnight() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 17).unwrap();

        // 23:30 + 60m = next day 00:30
        let pos = SlotPosition {
            date,
            time: NaiveTime::from_hms_opt(23, 30, 0).unwrap(),
        };
        let next = advance_slot(pos, 60);
        assert_eq!(next.date, NaiveDate::from_ymd_opt(2026, 3, 18).unwrap());
        assert_eq!(next.time, NaiveTime::from_hms_opt(0, 30, 0).unwrap());

        // 23:00 + 60m = next day 00:00 (exactly midnight)
        let pos = SlotPosition {
            date,
            time: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        };
        let next = advance_slot(pos, 60);
        assert_eq!(next.date, NaiveDate::from_ymd_opt(2026, 3, 18).unwrap());
        assert_eq!(next.time, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn test_slot_position_ordering() {
        let mon = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
        let tue = NaiveDate::from_ymd_opt(2026, 3, 17).unwrap();
        let t9 = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let t10 = NaiveTime::from_hms_opt(10, 0, 0).unwrap();

        assert!(SlotPosition { date: mon, time: t9 } < SlotPosition { date: mon, time: t10 });
        assert!(SlotPosition { date: mon, time: t10 } < SlotPosition { date: tue, time: t9 });
    }

    fn slot(date: NaiveDate, h: u32, m: u32) -> SlotPosition {
        SlotPosition {
            date,
            time: NaiveTime::from_hms_opt(h, m, 0).unwrap(),
        }
    }

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, day).unwrap()
    }

    #[gpui::test]
    fn test_schedule_view_state_builder(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                cx.set_global(crate::theme::Theme::default());
                let state = cx.new(|cx| {
                    ScheduleViewState::new(window, cx)
                        .interval(SlotInterval::minutes(15))
                        .num_days(7)
                        .visible_hours(8, 20)
                });

                let s = state.read(cx);
                assert_eq!(s.interval.as_minutes(), 15);
                assert_eq!(s.num_days, 7);
                assert_eq!(s.day_start_hour, 8);
                assert_eq!(s.day_end_hour, 20);
                assert_eq!(s.dates().len(), 7);
                assert!(s.drag.is_none());

                let slots = s.time_slots();
                assert_eq!(slots.len(), (20 - 8) * 4); // 12 hours * 4 slots/hour
                assert_eq!(slots[0], NaiveTime::from_hms_opt(8, 0, 0).unwrap());
                assert_eq!(slots.last().unwrap(), &NaiveTime::from_hms_opt(19, 45, 0).unwrap());

                cx.new(|cx| crate::Root::new(state.clone(), window, cx))
            })
            .unwrap();
        });
    }

    #[gpui::test]
    fn test_drag_range_same_day(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                cx.set_global(crate::theme::Theme::default());
                let state = cx.new(|cx| {
                    ScheduleViewState::new(window, cx)
                        .interval(SlotInterval::hours(1))
                        .num_days(5)
                });

                // Forward drag: 10:00 -> 12:00 on same day
                state.update(cx, |s, _| {
                    s.drag = Some((slot(d(17), 10, 0), slot(d(17), 12, 0)));
                });
                let range = state.read(cx).drag_range().unwrap();
                assert_eq!(range.start_date, d(17));
                assert_eq!(range.start, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
                assert_eq!(range.end_date, d(17));
                assert_eq!(range.end, NaiveTime::from_hms_opt(13, 0, 0).unwrap());

                // Reverse drag: 14:00 -> 10:00 (drag upward)
                state.update(cx, |s, _| {
                    s.drag = Some((slot(d(17), 14, 0), slot(d(17), 10, 0)));
                });
                let range = state.read(cx).drag_range().unwrap();
                assert_eq!(range.start, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
                assert_eq!(range.end, NaiveTime::from_hms_opt(15, 0, 0).unwrap());

                // Single slot click (anchor == extent)
                state.update(cx, |s, _| {
                    s.drag = Some((slot(d(17), 9, 0), slot(d(17), 9, 0)));
                });
                let range = state.read(cx).drag_range().unwrap();
                assert_eq!(range.start, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
                assert_eq!(range.end, NaiveTime::from_hms_opt(10, 0, 0).unwrap());

                cx.new(|cx| crate::Root::new(state.clone(), window, cx))
            })
            .unwrap();
        });
    }

    #[gpui::test]
    fn test_drag_range_cross_day(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                cx.set_global(crate::theme::Theme::default());
                let state = cx.new(|cx| {
                    ScheduleViewState::new(window, cx)
                        .interval(SlotInterval::hours(1))
                        .num_days(5)
                });

                // Forward drag across days: Mon 22:00 -> Tue 02:00
                state.update(cx, |s, _| {
                    s.drag = Some((slot(d(16), 22, 0), slot(d(17), 2, 0)));
                });
                let range = state.read(cx).drag_range().unwrap();
                assert_eq!(range.start_date, d(16));
                assert_eq!(range.start, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
                assert_eq!(range.end_date, d(17));
                assert_eq!(range.end, NaiveTime::from_hms_opt(3, 0, 0).unwrap());

                // Reverse drag across days: Tue 02:00 -> Mon 22:00
                state.update(cx, |s, _| {
                    s.drag = Some((slot(d(17), 2, 0), slot(d(16), 22, 0)));
                });
                let range = state.read(cx).drag_range().unwrap();
                assert_eq!(range.start_date, d(16));
                assert_eq!(range.start, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
                assert_eq!(range.end_date, d(17));
                assert_eq!(range.end, NaiveTime::from_hms_opt(3, 0, 0).unwrap());

                cx.new(|cx| crate::Root::new(state.clone(), window, cx))
            })
            .unwrap();
        });
    }

    #[gpui::test]
    fn test_is_slot_in_drag_range(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            cx.open_window(gpui::WindowOptions::default(), |window, cx| {
                cx.set_global(crate::theme::Theme::default());
                let state = cx.new(|cx| {
                    ScheduleViewState::new(window, cx)
                        .interval(SlotInterval::hours(1))
                        .num_days(5)
                });

                // Drag from Mon 22:00 to Tue 02:00
                state.update(cx, |s, _| {
                    s.drag = Some((slot(d(16), 22, 0), slot(d(17), 2, 0)));
                });

                let s = state.read(cx);
                // In range
                assert!(s.is_slot_in_drag_range(d(16), NaiveTime::from_hms_opt(22, 0, 0).unwrap()));
                assert!(s.is_slot_in_drag_range(d(16), NaiveTime::from_hms_opt(23, 0, 0).unwrap()));
                assert!(s.is_slot_in_drag_range(d(17), NaiveTime::from_hms_opt(0, 0, 0).unwrap()));
                assert!(s.is_slot_in_drag_range(d(17), NaiveTime::from_hms_opt(2, 0, 0).unwrap()));

                // Out of range
                assert!(!s.is_slot_in_drag_range(d(16), NaiveTime::from_hms_opt(21, 0, 0).unwrap()));
                assert!(!s.is_slot_in_drag_range(d(17), NaiveTime::from_hms_opt(3, 0, 0).unwrap()));
                assert!(!s.is_slot_in_drag_range(d(18), NaiveTime::from_hms_opt(0, 0, 0).unwrap()));

                cx.new(|cx| crate::Root::new(state.clone(), window, cx))
            })
            .unwrap();
        });
    }
}
