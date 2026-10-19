// The gallery's demo of Slider: keyboard and ranges.

import { signal, Text, View } from "nsh";
import { Slider } from "../../../../crates/nsh/types/gpui-base";
import { log, muted, Section } from "../look";

/** GPUI Kit's slider: a thin muted track, a primary range and round thumbs. */
const horizontal = {
  rootClass: "w_full h_5",
  trackClass: "flex items_center w_full h_5",
  indicatorClass: "w_full h_1p5 rounded_full bg_muted",
  rangeClass: "h_full rounded_full bg_primary",
};
const vertical = {
  rootClass: "h_32 w_5",
  trackClass: "flex justify_center h_full w_5",
  indicatorClass: "h_full w_1p5 rounded_full bg_muted",
  rangeClass: "w_full rounded_full bg_primary",
};
const thumb = {
  thumbClass: "size_4 rounded_full border_2 border_primary bg_background",
  thumbHoverClass: "size_5",
  thumbActiveClass: "size_5",
  /** The thumb the keys move, while the slider has focus. */
  thumbFocusClass: "border_ring",
};

const brightness = signal(60);
const price = signal<[number, number]>([200, 800]);
const level = signal(30);
const opacity = signal(0.5);

/** A value as the readout shows it. */
function show(value: number | [number, number]) {
  return Array.isArray(value) ? `${value[0]} – ${value[1]}` : `${value}`;
}

export function SliderDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Slider with the keyboard"
        note="Tab to a slider: the arrow keys move a step, Page Up and Page Down ten steps, Home and End to the ends."
      >
        <View style={{ width: 320, gap: 8 }}>
          <Slider
            value={brightness()}
            accessibilityLabel="Brightness"
            {...horizontal}
            {...thumb}
            onChange={(e) => brightness.set(e.value as number)}
            onRelease={(e) => log("Slider.release", e)}
          />
          <Text class={`${muted} text-xs`}>{`Brightness ${show(brightness())}`}</Text>
        </View>
        <View style={{ width: 320, gap: 8 }}>
          <Slider
            value={opacity()}
            max={1}
            step={0.05}
            accessibilityLabel="Opacity"
            {...horizontal}
            {...thumb}
            onChange={(e) => opacity.set(e.value as number)}
            onRelease={(e) => log("Slider.release", e)}
          />
          <Text class={`${muted} text-xs`}>{`Opacity ${show(opacity())}, in steps of 0.05`}</Text>
        </View>
      </Section>
      <Section
        title="Range"
        note="A [start, end] value has two thumbs. The keys move the thumb the pointer moved last."
      >
        <View style={{ width: 320, gap: 8 }}>
          <Slider
            value={price()}
            min={0}
            max={1000}
            step={10}
            accessibilityLabel="Price"
            {...horizontal}
            {...thumb}
            onChange={(e) => price.set(e.value as [number, number])}
            onRelease={(e) => log("Slider.release", e)}
          />
          <Text class={`${muted} text-xs`}>{`Price ${show(price())}`}</Text>
        </View>
      </Section>
      <Section title="Vertical and disabled" note="Up increases a vertical slider. A disabled slider is no Tab stop.">
        <Slider
          value={level()}
          vertical
          accessibilityLabel="Level"
          {...vertical}
          {...thumb}
          onChange={(e) => level.set(e.value as number)}
          onRelease={(e) => log("Slider.release", e)}
        />
        <Slider
          value={[20, 70]}
          vertical
          accessibilityLabel="Band"
          {...vertical}
          {...thumb}
          onChange={(e) => log("Slider.change", e)}
        />
        <Slider
          value={40}
          disabled
          accessibilityLabel="Locked"
          {...vertical}
          {...thumb}
          rangeClass="w_full rounded_full bg_muted_foreground opacity_50"
          thumbClass="size_4 rounded_full border_2 border_muted_foreground bg_background opacity_50"
        />
        <Text class={`${muted} text-xs`}>{`Level ${show(level())}`}</Text>
      </Section>
    </View>
  );
}
