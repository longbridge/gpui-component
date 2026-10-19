---
title: Calendar
description: A state-driven date grid with selection matchers and custom item rendering.
order: 5
---

# Calendar

A state-driven date grid with selection matchers and custom item rendering.

Like every `gpui-base` primitive, Calendar supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- calendar
```

## Import

```rust
use gpui_kit::base::{Calendar, CalendarState};
```

## Anatomy and API

The example composes `Calendar`, `CalendarState`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/calendar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/calendar.rs). Native and browser previews compile this same file.

## State and events

Selection lives in `CalendarState`; configure matching and update the state from calendar item interaction.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/calendar.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Keyboard

The calendar is one Tab stop. In the day view, the keyboard moves a focused day:

| Key | Action |
| --- | --- |
| Left / Right | Previous / next day |
| Up / Down | Previous / next week |
| PageUp / PageDown | Previous / next month, on the same day where it exists |
| Shift-PageUp / Shift-PageDown | Previous / next year |
| Home / End | Start / end of the week, following `first_day_of_week` |
| Enter / Space | Select the focused day |

The months shown follow the focused day. Moves by a day or a week skip disabled days; the other moves settle on the nearest enabled day, so with a disabled matcher such as `Matcher::interval(min, max)` the focus stops at the bounds. `CalendarItemState::is_focused` marks the focused day while the calendar has keyboard focus, for a focus ring; `CalendarState::focused_date` and `set_focused_date` read and move it. The bindings live in the `Calendar` key context.

## Accessibility

Each month is a grid of rows: weekday names are column headers and days are grid cells labeled with their ISO date, marked selected when picked or within the range and disabled when the matcher rejects them. The focused day is reported to assistive technology as the active descendant while the calendar holds focus.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
