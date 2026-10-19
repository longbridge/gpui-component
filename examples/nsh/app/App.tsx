// An NSH app the `example-nsh` example embeds in a GPUI Kit window: every
// GPUI Base control `gpui-nsh` registers, in five pages. Layout and text are
// NSH's; the controls are GPUI Base's, which own their behavior and draw
// nothing of their own, so the class strings below are their whole look:
// GPUI's style methods plus `bg_` / `text_` / `border_` with GPUI Base's
// color tokens, which follow the host's theme.

import { mount, Scroll, signal, Text, View } from "nsh";
import {
  text,
  muted,
  ring,
  button,
  buttonStates,
  accent,
  primary,
  field,
  trigger,
  triggerStates,
  popup,
  option,
  optionStates,
  segmented,
  segment,
  calendarLook,
  checkboxLook,
  lastEvent,
  log,
  Section,
} from "./look";
import { DisclosureDemo } from "./demos/disclosure";
import { OverlayDemo } from "./demos/overlay";
import { FloatingDemo } from "./demos/floating";
import { ToastDemo } from "./demos/toast";
import { DataDemo } from "./demos/data";
import { TreeDemo } from "./demos/tree";
import { ToolbarDemo } from "./demos/toolbar";
import { GroupsDemo } from "./demos/groups";
import { SliderDemo } from "./demos/slider";
import { SelectDemo } from "./demos/select";
import { PickersDemo } from "./demos/pickers";
import { CalendarDemo } from "./demos/calendar";
import {
  Avatar,
  Button,
  Calendar,
  Checkbox,
  ColorPicker,
  Combobox,
  DatePicker,
  Input,
  Link,
  NumberInput,
  OtpInput,
  Pagination,
  Progress,
  Radio,
  RadioGroup,
  Select,
  SelectableText,
  Slider,
  Switch,
  Tabs,
  Textarea,
  TextView,
  TimeField,
  Toggle,
  ToggleGroup,
} from "../../../crates/nsh/types/gpui-base";

const page = signal(0);

// ---------------------------------------------------------------------------
// Buttons: Button, Toggle, ToggleGroup, Link.

const bold = signal(false);
const align = signal(0);
const clicks = signal(0);

function Buttons() {
  return (
    <View style={{ gap: 20 }}>
      <Section title="Button" note="Click, or focus it with Tab and press Space or Enter.">
        <Button
          label={`Clicked ${clicks()} times`}
          rootClass={primary}
          hoverClass="opacity_90"
          activeClass="opacity_80"
          focusClass={ring}
          onClick={() => clicks.update((n) => n + 1)}
        />
        <Button label="Outline" rootClass={button} {...buttonStates} onClick={(e) => log("Button.click", e)} />
        <Button label="Disabled" disabled rootClass={button} disabledClass="opacity_50" />
      </Section>
      <Section title="Toggle">
        <Toggle
          label="Bold"
          pressed={bold()}
          onChange={(e) => {
            bold.set(e.pressed);
            log("Toggle.change", e);
          }}
          rootClass={button}
          pressedClass={accent}
          hoverClass={accent}
          focusClass={ring}
        />
        <Text class={`${text} ${bold() ? "font-bold" : ""}`}>{bold() ? "Bold text" : "Regular text"}</Text>
      </Section>
      <Section title="ToggleGroup">
        <ToggleGroup
          items={["Left", "Center", "Right"]}
          value={align()}
          onChange={(e) => {
            align.set(e.value);
            log("ToggleGroup.change", e);
          }}
          rootClass={segmented}
          toggleClass={segment}
          toggleHoverClass="text_foreground"
          toggleFocusClass={ring}
          pressedClass="bg_background text_foreground"
        />
      </Section>
      <Section title="Link" note="GPUI Base never opens the URL itself: `activate` hands it to the app.">
        <Link
          label="gpui-kit.com"
          href="https://gpui-kit.com"
          rootClass="text_primary text_sm underline"
          hoverClass="text_foreground"
          onActivate={(e) => log("Link.activate", e)}
        />
      </Section>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Choices: Switch, Checkbox, Radio, RadioGroup, Slider, Progress.

const alerts = signal(true);
const fruits = signal([true, false, false]);
const terms = signal(false);
const plan = signal(1);
const volume = signal(40);

function Choices() {
  const all = () => fruits().every(Boolean);
  const some = () => fruits().some(Boolean) && !all();
  return (
    <View style={{ gap: 20 }}>
      <Section title="Switch">
        <Switch
          label="Send alerts"
          checked={alerts()}
          onChange={(e) => {
            alerts.set(e.checked);
            log("Switch.change", e);
          }}
          rootClass="flex items_center gap_2 text_sm"
          focusClass="text_primary"
          trackClass="flex w_9 h_5 p_0p5 rounded_full bg_input"
          trackCheckedClass="justify_end bg_primary"
          thumbClass="size_4 rounded_full bg_background shadow_sm"
        />
        <Switch
          label="Disabled"
          disabled
          rootClass="flex items_center gap_2 text_sm text_muted_foreground"
          trackClass="flex w_9 h_5 p_0p5 rounded_full bg_muted"
          thumbClass="size_4 rounded_full bg_background"
        />
      </Section>
      <Section title="Checkbox" note="The first is indeterminate while only some fruits are chosen.">
        <View style={{ gap: 6 }}>
          <Checkbox
            label="All fruits"
            checked={all()}
            indeterminate={some()}
            onChange={(e) => fruits.set(fruits().map(() => e.checked))}
            {...checkboxLook}
          />
          {["Apple", "Banana", "Cherry"].map((fruit, i) => (
            <View style={{ paddingLeft: 24 }}>
              <Checkbox
                label={fruit}
                checked={fruits()[i]}
                onChange={(e) => {
                  fruits.set(fruits().map((v, j) => (j === i ? e.checked : v)));
                  log("Checkbox.change", e);
                }}
                {...checkboxLook}
              />
            </View>
          ))}
        </View>
      </Section>
      <Section title="Radio">
        <Radio
          label="I accept the terms"
          checked={terms()}
          onChange={(e) => {
            terms.set(e.checked);
            log("Radio.change", e);
          }}
          rootClass="flex items_center gap_2 text_sm"
          hoverClass="text_primary"
          checkedClass="font_semibold"
        />
      </Section>
      <Section title="RadioGroup">
        <RadioGroup
          items={["Free", "Pro", "Team"]}
          value={plan()}
          onChange={(e) => {
            plan.set(e.value);
            log("RadioGroup.change", e);
          }}
          axis="horizontal"
          rootClass="flex gap_2"
          radioClass="h_8 px_3 flex items_center rounded_md border_1 border_border text_sm"
          radioHoverClass={accent}
          radioFocusClass={ring}
          checkedClass={accent}
        />
      </Section>
      <Section title="Slider and Progress" note="The progress bar follows the slider.">
        <View style={{ width: 320, gap: 12 }}>
          <Slider
            value={volume()}
            onChange={(e) => volume.set(e.value)}
            onRelease={(e) => log("Slider.release", e)}
            rootClass="w_full h_5"
            trackClass="flex items_center w_full h_5"
            indicatorClass="w_full h_1p5 rounded_full bg_muted"
            rangeClass="h_full rounded_full bg_primary"
            thumbClass="size_4 rounded_full border_2 border_primary bg_background"
            thumbHoverClass="size_5"
          />
          <Progress
            value={volume()}
            rootClass="w_full"
            trackClass="w_full h_2 rounded_full bg_muted overflow_hidden"
            indicatorClass="h_full rounded_full bg_primary"
          />
          <Text class={`${muted} text-xs`}>{`${Math.round(volume())}%`}</Text>
        </View>
      </Section>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Fields: Input, Textarea, NumberInput, OtpInput, TimeField.

const name = signal("Ada");
const notes = signal("");
const quantity = signal("3");
const code = signal("");
const time = signal("09:30");

function Fields() {
  return (
    <View style={{ gap: 20, maxWidth: 420 }}>
      <Section title="Input" note="Enter submits.">
        <View style={{ width: 360 }}>
          <Input
            value={name()}
            placeholder="Your name"
            onChange={(e) => name.set(e.value)}
            onSubmit={(e) => log("Input.submit", e)}
            rootClass={field}
            hoverClass="border_ring"
            focusedClass={ring}
          />
        </View>
      </Section>
      <Section title="Textarea" note="Enter adds a line.">
        <View style={{ width: 360 }}>
          <Textarea
            value={notes()}
            placeholder="Notes"
            rows={3}
            onChange={(e) => notes.set(e.value)}
            rootClass="w_full px_2 py_1p5 rounded_md border_1 border_input bg_background text_sm"
            hoverClass="border_ring"
            focusedClass={ring}
          />
        </View>
      </Section>
      <Section title="NumberInput" note="From 0 to 10; the arrow keys step too.">
        <NumberInput
          value={quantity()}
          min={0}
          max={10}
          step={1}
          onChange={(e) => {
            quantity.set(e.value);
            log("NumberInput.change", e);
          }}
          decrementLabel="−"
          incrementLabel="+"
          rootClass="flex items_center h_8 w_32 rounded_md border_1 border_input bg_background"
          inputClass="flex_1 px_2 text_sm text_center"
          decrementClass="size_8 flex items_center justify_center"
          incrementClass="size_8 flex items_center justify_center"
          stepHoverClass="bg_accent"
        />
      </Section>
      <Section title="OtpInput" note="Focus it with Tab, then type six digits.">
        <OtpInput
          value={code()}
          length={6}
          onChange={(e) => code.set(e.value)}
          onComplete={(e) => log("OtpInput.complete", e)}
          rootClass="flex gap_1"
          cellClass="size_9 flex items_center justify_center rounded_md border_1 border_input text_lg"
          activeCellClass="border_ring"
        />
      </Section>
      <Section title="TimeField" note="Click a segment, then type or use the arrow keys.">
        <TimeField
          value={time()}
          hourCycle="h12"
          onChange={(e) => {
            time.set(e.value);
            log("TimeField.change", e);
          }}
          rootClass="flex items_center h_8 px_2 rounded_md border_1 border_input bg_background text_sm"
          segmentClass="px_0p5 rounded_sm"
          segmentHoverClass="bg_accent"
          selectedSegmentClass="bg_primary text_primary_foreground"
        />
      </Section>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Pickers: Select, Combobox, DatePicker, ColorPicker, Calendar.

const region = signal("Europe");
const regionOpen = signal(false);
const city = signal("");
const cityOpen = signal(false);
const cityQuery = signal("");
const date = signal("2026-10-19");
const dateOpen = signal(false);
const color = signal("#7C3AED");
const colorOpen = signal(false);
const range = signal("2026-10-12/2026-10-16");

function Pickers() {
  return (
    <View style={{ gap: 20 }}>
      <Section title="Select">
        <Select
          items={["Asia", "Europe", "Americas", "Africa", "Oceania"]}
          value={region()}
          open={regionOpen()}
          onOpenChange={(e) => regionOpen.set(e.open)}
          onChange={(e) => {
            region.set(e.value);
            regionOpen.set(false);
            log("Select.change", e);
          }}
          rootClass="w_48"
          triggerClass={trigger}
          {...triggerStates}
          contentClass={`w_48 ${popup}`}
          itemClass={option}
          {...optionStates}
        />
      </Section>
      <Section title="Combobox" note="Type to filter.">
        <Combobox
          items={["Hong Kong", "Singapore", "Shanghai", "London", "New York", "Tokyo"]}
          value={city()}
          placeholder="Choose a city"
          query={cityQuery()}
          queryPlaceholder="Search…"
          open={cityOpen()}
          onOpenChange={(e) => cityOpen.set(e.open)}
          onQueryChange={(e) => cityQuery.set(e.query)}
          onChange={(e) => {
            city.set(e.value);
            cityOpen.set(false);
            log("Combobox.change", e);
          }}
          rootClass="w_56"
          triggerClass={trigger}
          {...triggerStates}
          placeholderClass="text_muted_foreground"
          contentClass={`w_56 ${popup}`}
          inputClass="w_full h_8 px_2 mb_1 flex items_center rounded_md border_1 border_input text_sm"
          inputFocusedClass={ring}
          itemClass={option}
          {...optionStates}
        />
      </Section>
      <Section title="DatePicker">
        <DatePicker
          value={date()}
          open={dateOpen()}
          onOpenChange={(e) => dateOpen.set(e.open)}
          onChange={(e) => {
            date.set(e.value);
            dateOpen.set(false);
            log("DatePicker.change", e);
          }}
          placeholder="Pick a date"
          triggerClass={`w_48 ${trigger}`}
          {...triggerStates}
          popupClass={`p_2 ${popup}`}
          calendarClass="flex flex_col gap_1"
          {...calendarLook}
        />
      </Section>
      <Section title="ColorPicker">
        <ColorPicker
          value={color()}
          swatches={["#EF4444", "#F59E0B", "#10B981", "#3B82F6", "#7C3AED", "#EC4899"]}
          open={colorOpen()}
          onOpenChange={(e) => colorOpen.set(e.open)}
          onChange={(e) => {
            color.set(e.value);
            log("ColorPicker.change", e);
          }}
          triggerClass={trigger}
          {...triggerStates}
          previewClass="size_4 rounded_sm border_1 border_border"
          popupClass={`p_2 flex flex_col gap_2 ${popup}`}
          swatchesClass="flex gap_1"
          swatchClass="size_6 rounded_md"
          swatchHoverClass="border_2 border_ring"
          swatchSelectedClass="border_2 border_foreground"
          inputClass="w_32 h_7 px_2 flex items_center rounded_md border_1 border_input text_sm"
          inputFocusedClass={ring}
        />
      </Section>
      <Section title="Calendar" note="A range: click a start, then an end.">
        <Calendar
          mode="range"
          value={range()}
          onChange={(e) => {
            range.set(e.value);
            log("Calendar.change", e);
          }}
          rootClass="p_2 rounded_md border_1 border_border"
          calendarClass="flex flex_col gap_1"
          {...calendarLook}
        />
      </Section>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Content: Avatar, SelectableText, TextView, Pagination.

const pageNumber = signal(3);
const markdown = `## TextView

GPUI Base renders **Markdown** and *HTML*. A link such as [the docs](https://gpui-kit.com/docs) goes to the app as \`linkClick\`.

- Wrapped to the width NSH gives the box
- Selectable with the pointer`;

function Content() {
  return (
    <View style={{ gap: 20 }}>
      <Section title="Avatar" note="An image, or the fallback when there is none.">
        <Avatar
          src="https://avatars.githubusercontent.com/u/5518?v=4"
          fallback="JL"
          rootClass="size_10 rounded_full overflow_hidden bg_muted"
          imageClass="size_10"
          fallbackClass="size_10 flex items_center justify_center text_sm"
        />
        <Avatar
          fallback="AB"
          rootClass="size_10 rounded_full bg_primary text_primary_foreground"
          fallbackClass="size_10 flex items_center justify_center text_sm font_semibold"
        />
      </Section>
      <Section title="SelectableText" note="Drag across it, then copy.">
        <SelectableText text="GPUI Base's own selection, inside an NSH app." rootClass="text_sm" />
      </Section>
      <Section title="TextView">
        <View style={{ width: 520 }}>
          <TextView
            text={markdown}
            format="markdown"
            rootClass="w_full text_sm"
            onLinkClick={(e) => log("TextView.linkClick", e)}
          />
        </View>
      </Section>
      <Section title="Pagination">
        <Pagination
          value={pageNumber()}
          totalPages={12}
          onChange={(e) => {
            pageNumber.set(e.value);
            log("Pagination.change", e);
          }}
          previousLabel="‹"
          nextLabel="›"
          rootClass="flex items_center gap_1"
          pageClass="size_8 flex items_center justify_center rounded_md text_sm"
          previousClass="size_8 flex items_center justify_center rounded_md"
          nextClass="size_8 flex items_center justify_center rounded_md"
          buttonHoverClass="bg_accent"
          buttonFocusClass={ring}
          currentPageClass="border_1 border_border bg_background"
          ellipsisClass="px_1 text_muted_foreground"
          disabledClass="opacity_50"
        />
      </Section>
    </View>
  );
}

// ---------------------------------------------------------------------------

/** A page: its own controls, then the demos added to it. */
function stack(...views: (() => unknown)[]) {
  return () => <View style={{ gap: 20 }}>{views.map((view) => view())}</View>;
}

const pages = [
  { title: "Buttons", view: stack(Buttons, GroupsDemo, ToolbarDemo) },
  { title: "Choices", view: stack(Choices, SliderDemo) },
  { title: "Fields", view: stack(Fields) },
  { title: "Pickers", view: stack(Pickers, SelectDemo, PickersDemo, CalendarDemo) },
  { title: "Disclosure", view: stack(DisclosureDemo) },
  { title: "Overlays", view: stack(OverlayDemo, FloatingDemo, ToastDemo) },
  { title: "Data", view: stack(DataDemo, TreeDemo) },
  { title: "Content", view: stack(Content) },
];

function App() {
  return (
    <View style={{ flex: 1 }}>
      <View style={{ paddingHorizontal: 24, paddingTop: 20, paddingBottom: 12 }}>
        <Tabs
          items={pages.map((p) => p.title)}
          value={page()}
          onChange={(e) => page.set(e.value)}
          rootClass={segmented}
          tabClass={segment}
          tabHoverClass="text_foreground"
          selectedClass="bg_background text_foreground"
        />
      </View>
      <Scroll style={{ flex: 1 }}>
        <View style={{ paddingHorizontal: 24, paddingBottom: 24 }}>{pages[page()].view()}</View>
      </Scroll>
      <View class="border-t border-zinc-200 dark:border-zinc-800" style={{ paddingHorizontal: 24, paddingVertical: 8 }}>
        <Text class={`${muted} text-xs`}>{lastEvent()}</Text>
      </View>
    </View>
  );
}

mount(App);
