---
title: Calendar
description: A flexible calendar component for displaying months, navigating dates, and selecting single dates or date ranges.
---

# Calendar

A standalone calendar component that provides a rich interface for date selection and navigation. The Calendar component supports single date selection, date range selection, multiple month views, custom disabled dates, and comprehensive keyboard navigation.

## Import

```rust
use gpui_component::{
    calendar::{Calendar, CalendarState, CalendarEvent, Date, Matcher},
};
```

## Usage

### Basic Calendar

```rust
let calendar = cx.new(|cx| CalendarState::new(window, cx));

Calendar::new(&calendar)
```

### Calendar with Initial Date

```rust
use chrono::Local;

let calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Local::now().naive_local().date(), window, cx);
    state
});

Calendar::new(&calendar)
```

### Date Range Calendar

```rust
use chrono::{Local, Days};

let calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    let now = Local::now().naive_local().date();
    state.set_date(
        Date::Range(Some(now), now.checked_add_days(Days::new(7))),
        window,
        cx
    );
    state
});

Calendar::new(&calendar)
```

### Multiple Months Display

```rust
// Show 2 months side by side
Calendar::new(&calendar)
    .number_of_months(2)

// Show 3 months
Calendar::new(&calendar)
    .number_of_months(3)
```

### Calendar Sizes

```rust
Calendar::new(&calendar).large()
Calendar::new(&calendar) // medium (default)
Calendar::new(&calendar).small()
```

## Date Restrictions

### Disabled Weekends

```rust
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(vec![0, 6]) // Sunday=0, Saturday=6
});

Calendar::new(&calendar)
```

### Disabled Specific Weekdays

```rust
// Disable Sundays, Wednesdays, and Saturdays
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(vec![0, 3, 6])
});

Calendar::new(&calendar)
```

### Disabled Date Range

```rust
use chrono::{Local, Days};

let now = Local::now().naive_local().date();

let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::range(
            Some(now),
            now.checked_add_days(Days::new(7)),
        ))
});

Calendar::new(&calendar)
```

### Disabled Date Interval

```rust
// Disable dates outside the interval (before/after specified dates)
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::interval(
            Some(now.checked_sub_days(Days::new(30)).unwrap()),
            now.checked_add_days(Days::new(30))
        ))
});

Calendar::new(&calendar)
```

### Custom Disabled Dates

```rust
// Disable first 5 days of each month
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            date.day0() < 5 // day0() returns 0-based day
        }))
});

Calendar::new(&calendar)

// Disable all Mondays
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            date.weekday() == chrono::Weekday::Mon
        }))
});

Calendar::new(&calendar)

// Disable past dates
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            *date < Local::now().naive_local().date()
        }))
});

Calendar::new(&calendar)
```

## Month/Year Navigation

The Calendar automatically provides navigation controls:

- **Previous/Next Month**: Arrow buttons in the header
- **Month Selection**: Click on month name to open month picker
- **Year Selection**: Click on year to open year picker
- **Year Pages**: Navigate through 20-year pages in year view

### Custom Year Range

```rust
let calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .year_range((2020, 2030)) // Limit to specific year range
});

Calendar::new(&calendar)
```

## Handle Selection Events

```rust
let calendar = cx.new(|cx| CalendarState::new(window, cx));

cx.subscribe(&calendar, |view, _, event, _| {
    match event {
        CalendarEvent::Selected(date) => {
            match date {
                Date::Single(Some(selected_date)) => {
                    println!("Date selected: {}", selected_date);
                }
                Date::Range(Some(start), Some(end)) => {
                    println!("Range selected: {} to {}", start, end);
                }
                Date::Range(Some(start), None) => {
                    println!("Range start: {}", start);
                }
                _ => {
                    println!("Selection cleared");
                }
            }
        }
    }
});

Calendar::new(&calendar)
```

## Advanced Examples

### Business Days Only Calendar

```rust
use chrono::Weekday;

let business_calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(|date| {
            matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
        }))
});

Calendar::new(&business_calendar)
```

### Holiday Calendar

```rust
use chrono::NaiveDate;
use std::collections::HashSet;

// Define holidays
let holidays: HashSet<NaiveDate> = [
    NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), // New Year
    NaiveDate::from_ymd_opt(2024, 7, 4).unwrap(), // Independence Day
    NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(), // Christmas
].into_iter().collect();

let holiday_calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(move |date| {
            holidays.contains(date)
        }))
});

Calendar::new(&holiday_calendar)
```

### Multi-Month Range Selector

```rust
let range_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Date::Range(None, None), window, cx); // Range mode
    state
});

Calendar::new(&range_calendar)
    .number_of_months(3) // Show 3 months for easier range selection
```

### Quarterly View Calendar

```rust
let quarterly_calendar = cx.new(|cx| CalendarState::new(window, cx));

// Update to show current quarter's months
Calendar::new(&quarterly_calendar)
    .number_of_months(3)
```

## Custom Styling

```rust
use gpui::{px, relative};

Calendar::new(&calendar)
    .p_4() // Custom padding
    .bg(cx.theme().secondary) // Custom background
    .border_2() // Custom border
    .border_color(cx.theme().primary) // Custom border color
    .rounded(px(12.)) // Custom border radius
    .w(px(400.)) // Custom width
    .h(px(350.)) // Custom height
```

## API Reference

### CalendarState

| Method                                    | Description                         |
| ----------------------------------------- | ----------------------------------- |
| `new(window, cx)`                         | Create a new calendar state         |
| `disabled_matcher(matcher)`               | Set date disabling rules            |
| `set_date(date, window, cx)`              | Set selected date programmatically  |
| `date()`                                  | Get current selected date           |
| `set_number_of_months(count, window, cx)` | Set number of months to display     |
| `year_range(range)`                       | Set available year range (min, max) |

### Calendar

| Method                    | Description                              |
| ------------------------- | ---------------------------------------- |
| `new(state)`              | Create calendar with state entity        |
| `number_of_months(count)` | Set number of months to display          |
| `with_size(size)`         | Set calendar size (Small, Medium, Large) |
| `large()`                 | Large size variant                       |
| `small()`                 | Small size variant                       |

### Date

| Variant                                       | Description                |
| --------------------------------------------- | -------------------------- |
| `Single(Option<NaiveDate>)`                   | Single date selection mode |
| `Range(Option<NaiveDate>, Option<NaiveDate>)` | Date range selection mode  |

| Method               | Description                            |
| -------------------- | -------------------------------------- |
| `is_active(date)`    | Check if date is currently selected    |
| `is_single()`        | Check if in single date mode           |
| `is_in_range(date)`  | Check if date is within selected range |
| `is_some()`          | Check if any date is selected          |
| `is_complete()`      | Check if selection is complete         |
| `start()`            | Get start date of selection            |
| `end()`              | Get end date of selection              |
| `format(format_str)` | Format date(s) using chrono format     |

### Matcher

| Variant                                   | Description                                      |
| ----------------------------------------- | ------------------------------------------------ |
| `DayOfWeek(Vec<u32>)`                     | Disable specific weekdays (0=Sunday, 6=Saturday) |
| `Interval(IntervalMatcher)`               | Disable dates outside interval                   |
| `Range(RangeMatcher)`                     | Disable dates within range                       |
| `Custom(Box<dyn Fn(&NaiveDate) -> bool>)` | Custom disable function                          |

| Method                    | Description                            |
| ------------------------- | -------------------------------------- |
| `interval(before, after)` | Create interval matcher                |
| `range(from, to)`         | Create range matcher                   |
| `custom(fn)`              | Create custom matcher                  |
| `matched(date)`           | Check if date matches the matcher      |
| `date_matched(date)`      | Check if Date enum matches the matcher |

### CalendarEvent

| Event            | Description                     |
| ---------------- | ------------------------------- |
| `Selected(Date)` | Date or range selection changed |

## Size Variants

| Size     | Description       | Dimensions                       |
| -------- | ----------------- | -------------------------------- |
| `Small`  | Compact calendar  | Smaller spacing and text         |
| `Medium` | Default size      | Standard spacing and readability |
| `Large`  | Spacious calendar | Larger touch targets and text    |

## View Modes

The calendar supports three view modes:

1. **Day View**: Default monthly calendar grid
2. **Month View**: Grid of months for quick month selection
3. **Year View**: Grid of years for quick year selection

Users can navigate between views by clicking on the month/year buttons in the header.

## Keyboard Navigation

### Day View

- **Arrow Keys**: Navigate between dates
- **Enter**: Select current date
- **Escape**: Clear selection
- **Tab**: Navigate to header controls
- **Page Up/Down**: Previous/next month
- **Home**: Go to first day of month
- **End**: Go to last day of month

### Month View

- **Arrow Keys**: Navigate between months
- **Enter**: Select month and return to day view
- **Escape**: Return to day view

### Year View

- **Arrow Keys**: Navigate between years
- **Enter**: Select year and return to day view
- **Escape**: Return to day view
- **Page Up/Down**: Previous/next year page

### Header Navigation

- **Tab**: Navigate between prev/next buttons and month/year selectors
- **Enter/Space**: Activate buttons or toggle view modes

## Examples

### Event Planning Calendar

```rust
let event_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    // Disable past dates and weekends
    state = state.disabled_matcher(Matcher::custom(|date| {
        let now = Local::now().naive_local().date();
        *date < now || matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }));
    state
});

Calendar::new(&event_calendar)
    .large() // Easier to see and interact with
```

### Vacation Booking Calendar

```rust
let vacation_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx);
    state.set_date(Date::Range(None, None), window, cx); // Range mode
    state
});

Calendar::new(&vacation_calendar)
    .number_of_months(2) // Show 2 months for range selection
```

### Report Date Range Selector

```rust
let report_calendar = cx.new(|cx| {
    let mut state = CalendarState::new(window, cx)
        .year_range((2020, 2025)); // Limit to business years

    state.set_date(Date::Range(None, None), window, cx);
    state
});

Calendar::new(&report_calendar)
    .number_of_months(3)
    .small() // Compact for dashboard use
```

### Availability Calendar

```rust
use std::collections::HashSet;

let unavailable_dates: HashSet<NaiveDate> = get_unavailable_dates();

let availability_calendar = cx.new(|cx| {
    CalendarState::new(window, cx)
        .disabled_matcher(Matcher::custom(move |date| {
            unavailable_dates.contains(date)
        }))
});

Calendar::new(&availability_calendar)
    .number_of_months(2)
```

The Calendar component provides a foundation for any date-related UI requirements, from simple date pickers to complex scheduling interfaces.
