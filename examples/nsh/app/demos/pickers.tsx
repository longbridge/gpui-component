// The gallery's demo of DatePicker and ColorPicker: keyboard, dismissal and sliders.

import { signal, View } from "nsh";
import { ColorPicker, DatePicker } from "../../../../crates/nsh/types/gpui-base";
import { calendarLook, log, popup, ring, Section, trigger, triggerStates } from "../look";

const due = signal("");
const dueOpen = signal(false);
const brand = signal("#7C3AED");
const brandOpen = signal(false);

export function PickersDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="DatePicker: keyboard and dismissal"
        note="Tab to it and press Enter to open, Escape to close. Choosing a day or clicking outside closes it too."
      >
        <DatePicker
          value={due()}
          min="2026-10-01"
          max="2026-12-31"
          open={dueOpen()}
          onOpenChange={(e) => {
            dueOpen.set(e.open);
            log("DatePicker.openChange", e);
          }}
          onChange={(e) => {
            due.set(e.value);
            dueOpen.set(false);
            log("DatePicker.change", e);
          }}
          placeholder="Due date"
          accessibilityLabel="Due date"
          triggerClass={`w_48 ${trigger}`}
          {...triggerStates}
          triggerActiveClass="bg_muted"
          popupClass={`p_2 ${popup}`}
          calendarClass="flex flex_col gap_1"
          {...calendarLook}
        />
      </Section>
      <Section
        title="ColorPicker: swatches, hex and sliders"
        note="Hover a swatch to preview it. A swatch or Enter in the hex field commits and closes; the sliders commit and stay open."
      >
        <ColorPicker
          value={brand()}
          swatches={["#EF4444", "#F59E0B", "#10B981", "#0EA5E9", "#3B82F6", "#7C3AED", "#EC4899", "#18181B"]}
          sliders
          sliderLabels={["H", "S", "L", "A"]}
          open={brandOpen()}
          onOpenChange={(e) => {
            brandOpen.set(e.open);
            log("ColorPicker.openChange", e);
          }}
          onChange={(e) => {
            brand.set(e.value);
            log("ColorPicker.change", e);
          }}
          placeholder="No color"
          accessibilityLabel="Brand color"
          triggerClass={trigger}
          {...triggerStates}
          triggerActiveClass="bg_muted"
          previewClass="size_4 rounded_sm border_1 border_border"
          popupClass={`w_56 p_3 flex flex_col gap_3 ${popup}`}
          swatchesClass="flex flex_wrap gap_1"
          swatchClass="size_5 rounded_md border_1 border_border"
          swatchHoverClass="border_2 border_ring"
          swatchFocusClass="border_2 border_ring"
          swatchSelectedClass="border_2 border_foreground"
          slidersClass="flex flex_col gap_2"
          sliderRowClass="flex items_center gap_2"
          sliderLabelClass="w_3 text_xs text_muted_foreground"
          sliderClass="flex_1"
          sliderTrackClass="w_full h_4 flex items_center"
          sliderIndicatorClass="w_full h_1p5 rounded_full bg_muted"
          sliderRangeClass="rounded_full bg_primary"
          sliderThumbClass="size_3 rounded_full bg_background border_1 border_primary shadow_sm"
          sliderThumbHoverClass="border_2"
          inputClass="w_full h_7 px_2 flex items_center rounded_md border_1 border_input text_sm"
          inputFocusedClass={ring}
        />
      </Section>
    </View>
  );
}
