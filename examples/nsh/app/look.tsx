// The look and the helpers the gallery's pages share: class strings in GPUI
// Kit's style (GPUI Base draws nothing of its own), a section with a title,
// and the line that shows the last event a control sent.

import { signal, Text, View } from "nsh";

// ---------------------------------------------------------------------------
// The look: class strings shared by the controls.

/** NSH's own text is black by default; `dark:` follows the host's mode. */
export const text = "text-zinc-900 dark:text-zinc-100";
export const muted = "text-zinc-500 dark:text-zinc-400";

export const ring = "border_ring";
export const button = "h_8 px_3 flex items_center rounded_md border_1 border_border bg_background text_sm";
export const buttonStates = { hoverClass: "bg_accent", activeClass: "bg_muted", focusClass: ring };
/** GPUI Kit's pressed and selected look: the accent, as on hover. */
export const accent = "bg_accent text_accent_foreground";
export const primary = "h_8 px_3 flex items_center rounded_md border_1 border_primary bg_primary text_primary_foreground text_sm";
export const field = "w_full h_8 px_2 flex items_center rounded_md border_1 border_input bg_background text_sm";
export const trigger =
  "h_8 px_3 flex items_center justify_between gap_2 rounded_md border_1 border_input bg_background text_sm";
export const triggerStates = { triggerHoverClass: "bg_accent", triggerFocusedClass: ring };
export const popup = "mt_1 p_1 rounded_md border_1 border_border bg_background shadow_md";
export const option = "w_full h_7 px_2 flex items_center justify_start rounded_sm text_sm";
export const optionStates = {
  itemHoverClass: "bg_accent",
  itemFocusClass: "bg_accent",
  selectedItemClass: "text_primary font_semibold",
};
export const segmented = "flex gap_1 p_1 rounded_lg bg_muted";
export const segment = "h_7 px_3 flex items_center rounded_md text_sm text_muted_foreground";
export const calendarLook = {
  weekdayNames: ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
  monthNames: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
  previousLabel: "‹",
  nextLabel: "›",
  navClass: "size_7 flex items_center justify_center rounded_md",
  headingClass: "h_7 px_2 flex items_center rounded_md font_semibold text_sm",
  weekdayClass: "size_8 flex items_center justify_center text_xs text_muted_foreground",
  dayClass: "size_8 flex items_center justify_center rounded_md text_sm",
  cellClass: "h_8 px_2 flex items_center justify_center rounded_md text_sm",
  selectedClass: "bg_primary text_primary_foreground",
  todayClass: "border_1 border_border",
  mutedClass: "text_muted_foreground",
  inRangeClass: "bg_accent",
  itemDisabledClass: "text_muted_foreground opacity_50",
  itemHoverClass: "bg_accent",
};
export const checkboxLook = {
  rootClass: "flex items_center gap_2 text_sm",
  hoverClass: "text_primary",
  indicatorClass: "size_4 flex items_center justify_center rounded_sm border_1 border_primary",
  checkedIcon: "icons/check.svg",
  indeterminateIcon: "icons/minus.svg",
  iconClass: "size_3 text_primary_foreground",
  indicatorCheckedClass: "bg_primary",
  indicatorIndeterminateClass: "bg_muted_foreground",
};

// ---------------------------------------------------------------------------
// The page shown, and the last event any control sent.

export const lastEvent = signal("Use a control: its last event shows here.");
export function log(name: string, data: unknown) {
  lastEvent.set(`${name} ${JSON.stringify(data)}`);
}

export function Section(props: { title: string; note?: string; children: unknown }) {
  return (
    <View style={{ gap: 8 }}>
      <Text class={`${text} text-sm font-semibold`}>{props.title}</Text>
      {props.note ? <Text class={`${muted} text-xs`}>{props.note}</Text> : null}
      <View style={{ flexDirection: "row", flexWrap: "wrap", alignItems: "center", gap: 12 }}>{props.children}</View>
    </View>
  );
}

