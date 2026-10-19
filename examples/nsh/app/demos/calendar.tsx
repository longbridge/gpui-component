// The gallery's demo of Calendar: keyboard navigation in a single and a range
// calendar, two months side by side, and days outside `min`/`max` disabled.

import { signal, Text, View } from "nsh";

import { Calendar } from "../../../../crates/nsh/types/gpui-base";
import { calendarLook, log, muted, Section, text } from "../look";

/** The day the keyboard acts on: GPUI Kit's focus ring. */
const keyboardLook = { ...calendarLook, focusedClass: "border_2 border_ring" };

const day = signal("2026-10-19");
const stay = signal("2026-10-26/2026-10-30");

export function CalendarDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Calendar: keyboard"
        note="Tab to the calendar. Arrows move by a day or a week, PageUp/PageDown by a month (with Shift a year), Home/End to the week's start and end; Enter or Space selects. Days before October 5 or after December 20 are disabled and skipped."
      >
        <Calendar
          value={day()}
          min="2026-10-05"
          max="2026-12-20"
          firstDayOfWeek="monday"
          onChange={(e) => {
            day.set(e.value);
            log("Calendar.change", e);
          }}
          rootClass="p_2 rounded_md border_1 border_border"
          calendarClass="flex flex_col gap_1"
          {...keyboardLook}
        />
        <Text class={`${muted} text-xs`}>{`Selected: ${day()}`}</Text>
      </Section>
      <Section
        title="Calendar: a range over two months"
        note="Enter on a start, move, Enter on an end. The months shown follow the focused day; nothing before today can be booked."
      >
        <Calendar
          mode="range"
          numberOfMonths={2}
          value={stay()}
          min="2026-10-19"
          onChange={(e) => {
            stay.set(e.value);
            log("Calendar.change", e);
          }}
          rootClass="w_128 p_2 rounded_md border_1 border_border"
          calendarClass="flex flex_col gap_1"
          {...keyboardLook}
        />
        <Text class={`${text} text-xs`}>{`Stay: ${stay().replace("/", " to ")}`}</Text>
      </Section>
    </View>
  );
}
