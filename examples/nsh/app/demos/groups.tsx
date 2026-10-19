// The gallery's demo of keyboard navigation of RadioGroup, ToggleGroup and Tabs:
// each group is one Tab stop, and the arrow keys move between its items.

import { signal, Text, View } from "nsh";

import { RadioGroup, Tabs, ToggleGroup } from "../../../../crates/nsh/types/gpui-base";
import { accent, log, muted, ring, Section, segment, segmented, text } from "../look";

const size = signal(1);
const view = signal(0);
const tools = signal(-1);
const page = signal(0);
const setting = signal(0);

/** A bordered row, GPUI Kit's radio-card look; the checked one shows the accent. */
const radioLook = {
  radioClass: "w_40 h_8 px_3 flex items_center rounded_md border_1 border_border text_sm",
  radioHoverClass: accent,
  radioFocusClass: ring,
  checkedClass: accent,
  disabledClass: "opacity_50",
};

/** GPUI Kit's underline tabs: a muted label, and the selected tab underlined in the primary color. */
const underlineTabs = {
  rootClass: "flex border_b_1 border_border",
  tabClass: "h_8 px_3 border_b_2 border_background text_sm text_muted_foreground",
  tabHoverClass: "text_foreground",
  tabFocusClass: "bg_accent text_accent_foreground rounded_t_md",
  selectedClass: "border_primary text_foreground",
  disabledClass: "opacity_50",
};

/** The same tabs down the side of a settings page. */
const sideTabs = {
  rootClass: "flex flex_col w_32 border_r_1 border_border",
  tabClass: "h_8 px_3 justify_start border_r_2 border_background text_sm text_muted_foreground",
  tabHoverClass: "text_foreground",
  tabFocusClass: "bg_accent text_accent_foreground rounded_l_md",
  selectedClass: "border_primary text_foreground",
};

const pages = ["Overview", "Activity", "Archived", "Settings"];
const settings = ["General", "Appearance", "Keyboard", "Privacy"];

function Panel(props: { title: string }) {
  return (
    <View style={{ paddingTop: 8 }}>
      <Text class={`${text} text-sm`}>{`${props.title} goes here.`}</Text>
    </View>
  );
}

export function GroupsDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Keyboard: RadioGroup"
        note="Tab lands on the checked radio. Every arrow key moves to the next or previous radio and checks it, as platform radio groups do; the disabled one is skipped."
      >
        <RadioGroup
          items={["Small", "Medium", "Large", "Extra large"]}
          value={size()}
          disabledItems={[2]}
          onChange={(e) => {
            size.set(e.value);
            log("RadioGroup.change", e);
          }}
          rootClass="flex flex_col gap_1"
          {...radioLook}
        />
      </Section>
      <Section
        title="Keyboard: ToggleGroup"
        note="Left and Right (Up and Down when vertical) move focus without pressing; Home and End go to the ends; Space or Enter presses. Tab comes back to the toggle focused last."
      >
        <ToggleGroup
          items={["List", "Grid", "Columns", "Gallery"]}
          value={view()}
          onChange={(e) => {
            view.set(e.value);
            log("ToggleGroup.change", e);
          }}
          rootClass={segmented}
          toggleClass={segment}
          toggleHoverClass="text_foreground"
          toggleFocusClass={ring}
          pressedClass="bg_background text_foreground"
        />
        <ToggleGroup
          items={["Select", "Draw", "Erase"]}
          value={tools()}
          axis="vertical"
          disabledItems={[2]}
          onChange={(e) => {
            tools.set(e.value);
            log("ToggleGroup.change", e);
          }}
          rootClass="flex flex_col gap_1"
          toggleClass="w_28 h_8 px_3 flex items_center justify_start rounded_md border_1 border_border text_sm"
          toggleHoverClass={accent}
          toggleFocusClass={ring}
          pressedClass={accent}
          disabledClass="opacity_50"
        />
      </Section>
      <Section
        title="Keyboard: Tabs"
        note="Tab lands on the selected tab; Left and Right select the next tab as focus moves, Home and End the first and last. A click selects without taking focus."
      >
        <View style={{ width: 420 }}>
          <Tabs
            items={pages}
            value={page()}
            disabledItems={[2]}
            onChange={(e) => {
              page.set(e.value);
              log("Tabs.change", e);
            }}
            {...underlineTabs}
          />
          <Panel title={pages[page()]} />
        </View>
      </Section>
      <Section
        title="Keyboard: vertical Tabs, activated with Enter"
        note="With `activateOnFocus` off, Up and Down only move focus; Space or Enter selects the focused tab, for content that is slow to show."
      >
        <View style={{ flexDirection: "row", gap: 16 }}>
          <Tabs
            items={settings}
            value={setting()}
            axis="vertical"
            activateOnFocus={false}
            onChange={(e) => {
              setting.set(e.value);
              log("Tabs.change", e);
            }}
            {...sideTabs}
          />
          <View style={{ width: 240 }}>
            <Text class={`${muted} text-xs`}>{settings[setting()]}</Text>
            <Panel title={`The ${settings[setting()].toLowerCase()} settings`} />
          </View>
        </View>
      </Section>
    </View>
  );
}
