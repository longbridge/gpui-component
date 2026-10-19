//! Date and time controls: TimeField, Calendar.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    App, AppContext as _, ElementId, Entity, Focusable as _, InteractiveElement as _,
    IntoElement as _, Refineable as _, SharedString, StyleRefinement, Subscription,
    base::{
        Calendar, CalendarEvent, CalendarItemKind, CalendarState, Date, HourCycle, Matcher,
        StyledExt as _, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision,
    },
    prelude::FluentBuilder as _,
};
use serde_json::json;

use crate::{Control, PropType, Props, catalog::class_style};

pub(super) fn controls() -> Vec<Control> {
    vec![time_field(), calendar()]
}

/// The style of class prop `name`, for a part Base builds in a slot.
/// The interaction-state class props of each time field segment; the field
/// holds the focus, shown by `selectedSegmentClass`.
const SEGMENT_STATES: [&str; 3] = ["segmentHoverClass", "", ""];

fn part_style(props: &Props, name: &str, cx: &App) -> StyleRefinement {
    props
        .string(name)
        .map(|class| class_style(&class, cx))
        .unwrap_or_default()
}

struct TimeFieldControl {
    state: Entity<TimeFieldState>,
    /// The precision the value is sent at.
    precision: Rc<Cell<TimePrecision>>,
    _subscription: Subscription,
}

fn time_field() -> Control {
    Control::stateful(
        "TimeField",
        |_, events, window, cx| {
            let state = cx.new(|cx| TimeFieldState::new(window, cx));
            let precision = Rc::new(Cell::new(TimePrecision::default()));
            let _subscription = cx.subscribe(&state, {
                let events = events.clone();
                let precision = precision.clone();
                move |_, TimeFieldEvent::Change(time), _| {
                    // The value is on a 24-hour clock whatever the field shows.
                    let format = precision.get().format(HourCycle::H23);
                    events.emit(
                        "change",
                        json!({ "value": time.format(format).to_string() }),
                    );
                }
            });
            TimeFieldControl {
                state,
                precision,
                _subscription,
            }
        },
        |control, props, _, window, cx| {
            let precision = match props.string("precision").as_deref() {
                Some("second") => TimePrecision::Second,
                _ => TimePrecision::Minute,
            };
            let hour_cycle = match props.string("hourCycle").as_deref() {
                Some("h12") => HourCycle::H12,
                _ => HourCycle::H23,
            };
            let time = props
                .changed("value")
                .then(|| props.string("value").and_then(|value| value.parse().ok()))
                .flatten();
            control.precision.set(precision);
            control.state.update(cx, |state, cx| {
                if props.changed("precision") {
                    state.set_precision(precision, window, cx);
                }
                if props.changed("hourCycle") {
                    state.set_hour_cycle(hour_cycle, window, cx);
                }
                if let Some(time) = time {
                    state.set_time(time, window, cx);
                }
            });
            let disabled = props.boolean("disabled");
            let segment = part_style(props, "segmentClass", cx);
            let selected = part_style(props, "selectedSegmentClass", cx);
            let states = props.state_styles(SEGMENT_STATES, cx);
            props
                .styled(TimeField::new(props.id(), &control.state), cx)
                .when(disabled, |this| props.styled_by("disabledClass", this, cx))
                .disabled(disabled)
                .render_segment(move |item, state, _, _| {
                    states
                        .clone()
                        .apply(item.refine_style(&segment))
                        .when(state.is_selected(), |this| this.refine_style(&selected))
                        .into_any_element()
                })
                .into_any_element()
        },
    )
    .focus(|control: &TimeFieldControl, cx| control.state.read(cx).focus_handle(cx))
    .documentation(
        "GPUI Base's TimeField: one Tab stop of hour, minute, second and AM/PM segments; \
         Up/Down step a segment, Left/Right and Tab move between them, digits type into them. \
         The value is `HH:MM`, or `HH:MM:SS` with second precision, on a 24-hour clock.",
    )
    .prop("value", PropType::String)
    .prop("precision", PropType::Enum(&["minute", "second"]))
    .prop("hourCycle", PropType::Enum(&["h23", "h12"]))
    .prop("disabled", PropType::Boolean)
    .prop("rootClass", PropType::String)
    .prop("segmentClass", PropType::String)
    .states(SEGMENT_STATES)
    .prop("selectedSegmentClass", PropType::String)
    .prop("disabledClass", PropType::String)
    .event("change", &[("value", PropType::String)])
}

/// The styles of a calendar's items, read from the class props once per
/// frame so the calendar's item slot can apply them.
struct CalendarStyles {
    nav: StyleRefinement,
    heading: StyleRefinement,
    weekday: StyleRefinement,
    day: StyleRefinement,
    cell: StyleRefinement,
    active: StyleRefinement,
    today: StyleRefinement,
    muted: StyleRefinement,
    in_range: StyleRefinement,
    disabled: StyleRefinement,
    hover: StyleRefinement,
}

impl CalendarStyles {
    fn new(props: &Props, cx: &App) -> Self {
        let style = |name| part_style(props, name, cx);
        Self {
            nav: style("navClass"),
            heading: style("headingClass"),
            weekday: style("weekdayClass"),
            day: style("dayClass"),
            cell: style("cellClass"),
            active: style("selectedClass"),
            today: style("todayClass"),
            muted: style("mutedClass"),
            in_range: style("inRangeClass"),
            disabled: style("itemDisabledClass"),
            hover: style("itemHoverClass"),
        }
    }

    fn part(&self, kind: CalendarItemKind) -> &StyleRefinement {
        match kind {
            CalendarItemKind::Previous | CalendarItemKind::Next => &self.nav,
            CalendarItemKind::MonthToggle | CalendarItemKind::YearToggle => &self.heading,
            CalendarItemKind::Weekday => &self.weekday,
            CalendarItemKind::Day => &self.day,
            CalendarItemKind::Month | CalendarItemKind::Year => &self.cell,
        }
    }
}

/// Base's Calendar `id` over `state`, mapped from the calendar props that
/// [`calendar_props`] declares: `calendarClass` on the root, the item class
/// props on its items, and the names and labels on their text.
pub(super) fn calendar_element(
    id: impl Into<ElementId>,
    state: &Entity<CalendarState>,
    props: &Props,
    cx: &App,
) -> Calendar {
    let styles = Rc::new(CalendarStyles::new(props, cx));
    let months = props.strings("monthNames");
    let weekdays = props.strings("weekdayNames");
    let previous = props.string("previousLabel");
    let next = props.string("nextLabel");
    props
        .styled_by("calendarClass", Calendar::new(id, state), cx)
        .number_of_months(props.number("numberOfMonths").max(1.) as usize)
        .when_some(
            props
                .string("firstDayOfWeek")
                .and_then(|day| day.parse().ok()),
            |this, day| this.first_day_of_week(day),
        )
        .item(move |item, state, _, _| {
            item.refine_style(styles.part(state.kind()))
                .when(state.is_muted(), |this| this.refine_style(&styles.muted))
                .when(state.is_today(), |this| this.refine_style(&styles.today))
                .when(state.is_in_range(), |this| {
                    this.refine_style(&styles.in_range)
                })
                .when(state.is_active(), |this| this.refine_style(&styles.active))
                .map(|this| {
                    if state.is_disabled() {
                        this.refine_style(&styles.disabled)
                    } else {
                        let hover = styles.hover.clone();
                        this.hover(move |mut style| {
                            style.refine(&hover);
                            style
                        })
                    }
                })
                .into_any_element()
        })
        .label(move |kind, value| {
            let name = |names: &[SharedString], index: i32| {
                usize::try_from(index)
                    .ok()
                    .and_then(|index| names.get(index).cloned())
            };
            match kind {
                CalendarItemKind::Previous => previous.clone().or(Some("‹".into())),
                CalendarItemKind::Next => next.clone().or(Some("›".into())),
                CalendarItemKind::MonthToggle | CalendarItemKind::Month => name(&months, value - 1),
                CalendarItemKind::Weekday => name(&weekdays, value),
                _ => None,
            }
            .unwrap_or_else(|| value.to_string().into())
        })
}

/// Declares the props [`calendar_element`] and [`sync_calendar`] read.
pub(super) fn calendar_props(control: Control) -> Control {
    control
        .prop("value", PropType::String)
        .prop("min", PropType::String)
        .prop("max", PropType::String)
        .prop("numberOfMonths", PropType::Number)
        .prop(
            "firstDayOfWeek",
            PropType::Enum(&[
                "sunday",
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
            ]),
        )
        .prop("monthNames", PropType::Strings)
        .prop("weekdayNames", PropType::Strings)
        .prop("previousLabel", PropType::String)
        .prop("nextLabel", PropType::String)
        .prop("calendarClass", PropType::String)
        .prop("navClass", PropType::String)
        .prop("headingClass", PropType::String)
        .prop("weekdayClass", PropType::String)
        .prop("dayClass", PropType::String)
        .prop("cellClass", PropType::String)
        .prop("selectedClass", PropType::String)
        .prop("todayClass", PropType::String)
        .prop("mutedClass", PropType::String)
        .prop("inRangeClass", PropType::String)
        .prop("itemDisabledClass", PropType::String)
        .prop("itemHoverClass", PropType::String)
}

/// An ISO date (`2025-02-10`), or a range of two (`2025-02-10/2025-02-14`);
/// empty while no date is picked.
pub(super) fn date_value(date: Date) -> String {
    match (date, date.start(), date.end()) {
        (Date::Range(..), Some(start), Some(end)) => format!("{start}/{end}"),
        (_, Some(start), _) => start.to_string(),
        _ => String::new(),
    }
}

/// The [`Date`] of an ISO date, or with `range` of two joined by `/`.
pub(super) fn parse_date(value: Option<&str>, range: bool) -> Date {
    let value = value.unwrap_or_default();
    if range {
        match value.split_once('/') {
            Some((start, end)) => Date::Range(start.parse().ok(), end.parse().ok()),
            None => Date::Range(None, None),
        }
    } else {
        Date::Single(value.parse().ok())
    }
}

/// Brings a [`CalendarState`] up to date with the `value`, `min` and `max`
/// props that JS set since the last frame: a range half picked in the
/// calendar is kept until the app sets another value. Days before `min` or
/// after `max` are disabled; `range` reads `value` as a range (see
/// [`parse_date`]), and a change of the `mode` prop applies `value` again.
pub(super) fn sync_calendar(
    state: &Entity<CalendarState>,
    props: &Props,
    range: bool,
    cx: &mut App,
) {
    if props.changed("min") || props.changed("max") {
        let date = |name| props.string(name).and_then(|date| date.parse().ok());
        let (min, max) = (date("min"), date("max"));
        let matcher =
            (min.is_some() || max.is_some()).then(|| Rc::new(Matcher::interval(min, max)));
        state.update(cx, |state, cx| {
            state.set_disabled_matcher_shared(matcher);
            cx.notify();
        });
    }
    if props.changed("value") || props.changed("mode") {
        let date = parse_date(props.string("value").as_deref(), range);
        state.update(cx, |state, cx| {
            if state.apply_date(date) {
                cx.notify();
            }
        });
    }
}

struct CalendarControl {
    state: Entity<CalendarState>,
    _subscription: Subscription,
}

fn calendar() -> Control {
    let control = Control::stateful(
        "Calendar",
        |_, events, window, cx| {
            let state = cx.new(|cx| CalendarState::new(window, cx));
            let _subscription = cx.subscribe(&state, {
                let events = events.clone();
                move |_, CalendarEvent::Selected(date), _| {
                    events.emit("change", json!({ "value": date_value(*date) }));
                }
            });
            CalendarControl {
                state,
                _subscription,
            }
        },
        |control, props, _, _, cx| {
            let range = props.string("mode").as_deref() == Some("range");
            sync_calendar(&control.state, props, range, cx);
            props
                .styled(calendar_element(props.id(), &control.state, props, cx), cx)
                .into_any_element()
        },
    )
    .documentation(
        "GPUI Base's Calendar: a month grid with month and year pickers; clicking a day \
         selects it. The value is an ISO date (`2025-02-10`), or with `mode: \"range\"` two \
         joined by `/` (`2025-02-10/2025-02-14`), sent once both ends are picked. Days \
         before `min` or after `max` are disabled.",
    )
    .prop("mode", PropType::Enum(&["single", "range"]))
    .prop("rootClass", PropType::String);
    calendar_props(control).event("change", &[("value", PropType::String)])
}

#[cfg(test)]
mod tests {
    use gpui_kit::{ElementId, TestAppContext, px};
    use serde_json::json;

    use crate::testing::{Fixture, TestWindowExt as _};

    fn id(name: &str) -> ElementId {
        ElementId::Name(name.to_owned().into())
    }

    #[gpui_kit::test]
    fn test_time_field_builder(cx: &mut TestAppContext) {
        let field = Fixture::new(
            cx,
            "TimeField",
            json!({
                "value": "21:05:30",
                "precision": "second",
                "hourCycle": "h12",
                "rootClass": "flex items_center gap_1 px_2 border_1 border_border rounded_md",
                "segmentClass": "px_1 rounded_sm",
                "selectedSegmentClass": "bg_accent",
                "disabledClass": "opacity_50",
            }),
        );
        let size = field.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        field.update(cx, |window, _| {
            assert_eq!(window.find(field.id.clone()).value(), Some("09:05:30 PM"));
            assert!(window.try_find("period").is_some());
        });
        assert!(field.focus_handle(cx).is_some());
        assert!(field.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_time_field_typing_and_keys(cx: &mut TestAppContext) {
        let field = Fixture::new(cx, "TimeField", json!({ "value": "09:30" }));
        field.update(cx, |window, cx| {
            window.click("hour", cx);
            window.press("1", cx);
            window.press("4", cx);
        });
        assert_eq!(
            field.take_events(),
            vec![
                ("change".into(), json!({ "value": "01:30" })),
                ("change".into(), json!({ "value": "14:30" })),
            ]
        );
        // Typing a complete hour moved on to the minute.
        field.update(cx, |window, cx| {
            window.press("up", cx);
            window.press("left", cx);
            window.press("down", cx);
        });
        assert_eq!(
            field.take_events(),
            vec![
                ("change".into(), json!({ "value": "14:31" })),
                ("change".into(), json!({ "value": "13:31" })),
            ]
        );
        field.update(cx, |window, _| {
            assert_eq!(window.find(field.id.clone()).value(), Some("13:31"));
        });
    }

    #[gpui_kit::test]
    fn test_time_field_seconds_value(cx: &mut TestAppContext) {
        let field = Fixture::new(
            cx,
            "TimeField",
            json!({ "value": "09:30:15", "precision": "second" }),
        );
        field.update(cx, |window, cx| {
            window.click("second", cx);
            window.press("up", cx);
        });
        assert_eq!(
            field.take_events(),
            vec![("change".into(), json!({ "value": "09:30:16" }))]
        );
    }

    #[gpui_kit::test]
    fn test_time_field_controlled(cx: &mut TestAppContext) {
        let field = Fixture::new(cx, "TimeField", json!({ "value": "09:30" }));
        field.set_prop(cx, "value", Some(json!("18:45")));
        field.update(cx, |window, _| {
            assert_eq!(window.find(field.id.clone()).value(), Some("18:45"));
        });
        field.set_prop(cx, "hourCycle", Some(json!("h12")));
        field.update(cx, |window, _| {
            assert_eq!(window.find(field.id.clone()).value(), Some("06:45 PM"));
        });
        assert!(field.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_time_field_disabled(cx: &mut TestAppContext) {
        let field = Fixture::new(
            cx,
            "TimeField",
            json!({ "value": "09:30", "disabled": true }),
        );
        field.update(cx, |window, cx| {
            window.click("hour", cx);
            window.press("5", cx);
            window.press("up", cx);
            assert_eq!(window.find(field.id.clone()).disabled(), Some(true));
        });
        assert!(field.take_events().is_empty());
        field.set_prop(cx, "disabled", None);
        field.update(cx, |window, cx| {
            window.click("hour", cx);
            window.press("5", cx);
        });
        assert_eq!(
            field.take_events(),
            vec![("change".into(), json!({ "value": "05:30" }))]
        );
    }

    // February 2025 starts on a Saturday: its second week (index 2 in the
    // grid that starts on Sunday, January 26) holds the 9th to the 15th.
    const FEB_10: &str = "calendar-2025-02-10-0-2";
    const FEB_12: &str = "calendar-2025-02-12-0-2";

    #[gpui_kit::test]
    fn test_calendar_builder(cx: &mut TestAppContext) {
        let calendar = Fixture::new(
            cx,
            "Calendar",
            json!({
                "value": "2025-02-10",
                "min": "2025-02-01",
                "max": "2025-02-28",
                "firstDayOfWeek": "sunday",
                "weekdayNames": ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
                "monthNames": ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                                "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
                "previousLabel": "Previous",
                "nextLabel": "Next",
                "rootClass": "gap_2 p_3",
                "calendarClass": "rounded_md",
                "navClass": "size_7",
                "headingClass": "px_2",
                "weekdayClass": "size_8 text_muted_foreground",
                "dayClass": "size_8 rounded_md",
                "cellClass": "h_8 px_2",
                "selectedClass": "bg_primary text_primary_foreground",
                "inRangeClass": "bg_accent",
                "mutedClass": "text_muted_foreground",
                "todayClass": "border_1 border_border",
                "itemDisabledClass": "opacity_50",
                "itemHoverClass": "bg_accent",
            }),
        );
        let size = calendar.measure(cx);
        assert!(size.width > px(0.) && size.height > px(0.));
        calendar.update(cx, |window, _| {
            assert_eq!(window.find(id(FEB_10)).label(), Some("2025-02-10"));
            assert_eq!(window.find("calendar-MonthToggle").label(), Some("Feb"));
            assert_eq!(window.find("weekday-0-1").label(), Some("Mo"));
            assert_eq!(window.find("calendar-prev").label(), Some("Previous"));
        });
    }

    #[gpui_kit::test]
    fn test_calendar_click_day(cx: &mut TestAppContext) {
        let calendar = Fixture::new(cx, "Calendar", json!({ "value": "2025-02-10" }));
        calendar.update(cx, |window, cx| window.click(id(FEB_12), cx));
        assert_eq!(
            calendar.take_events(),
            vec![("change".into(), json!({ "value": "2025-02-12" }))]
        );
        // The month buttons page through months; a day there selects it.
        calendar.update(cx, |window, cx| window.click("calendar-next", cx));
        calendar.update(cx, |window, cx| {
            window.click(id("calendar-2025-03-04-0-1"), cx)
        });
        assert_eq!(
            calendar.take_events(),
            vec![("change".into(), json!({ "value": "2025-03-04" }))]
        );
    }

    #[gpui_kit::test]
    fn test_calendar_range(cx: &mut TestAppContext) {
        let calendar = Fixture::new(
            cx,
            "Calendar",
            json!({ "mode": "range", "value": "2025-02-01/2025-02-03" }),
        );
        calendar.update(cx, |window, cx| window.click(id(FEB_10), cx));
        // Half a range is not a value yet.
        assert!(calendar.take_events().is_empty());
        calendar.update(cx, |window, cx| window.click(id(FEB_12), cx));
        assert_eq!(
            calendar.take_events(),
            vec![("change".into(), json!({ "value": "2025-02-10/2025-02-12" }))]
        );
    }

    #[gpui_kit::test]
    fn test_calendar_controlled(cx: &mut TestAppContext) {
        let calendar = Fixture::new(
            cx,
            "Calendar",
            json!({ "value": "2025-02-10", "dayClass": "w_8", "selectedClass": "w_16" }),
        );
        let width = |calendar: &Fixture, cx: &mut TestAppContext, day: &str| {
            calendar.update(cx, |window, _| window.find(id(day)).bounds().size.width)
        };
        assert_eq!(width(&calendar, cx, FEB_10), px(64.));
        assert_eq!(width(&calendar, cx, FEB_12), px(32.));
        calendar.set_prop(cx, "value", Some(json!("2025-02-12")));
        assert_eq!(width(&calendar, cx, FEB_10), px(32.));
        assert_eq!(width(&calendar, cx, FEB_12), px(64.));
        // Another month's date shows that month.
        calendar.set_prop(cx, "value", Some(json!("2025-03-04")));
        assert_eq!(width(&calendar, cx, "calendar-2025-03-04-0-1"), px(64.));
        assert!(calendar.take_events().is_empty());
    }

    #[gpui_kit::test]
    fn test_calendar_disabled_days(cx: &mut TestAppContext) {
        let calendar = Fixture::new(
            cx,
            "Calendar",
            json!({ "value": "2025-02-10", "min": "2025-02-05" }),
        );
        calendar.update(cx, |window, cx| {
            window.click(id("calendar-2025-02-03-0-1"), cx)
        });
        assert!(calendar.take_events().is_empty());
        calendar.set_prop(cx, "min", None);
        calendar.update(cx, |window, cx| {
            window.click(id("calendar-2025-02-03-0-1"), cx)
        });
        assert_eq!(
            calendar.take_events(),
            vec![("change".into(), json!({ "value": "2025-02-03" }))]
        );
    }
}
