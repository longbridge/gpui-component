// The gallery's demo of Accordion and Collapsible.

import { signal, View } from "nsh";
import { Accordion, Collapsible } from "../../../../crates/nsh/types/gpui-base";
import { log, Section } from "../look";

// GPUI Kit's Accordion: a bordered card of items split by rules (each item
// draws its bottom one, the last closing the card), a medium
// title row with a muted chevron, and the panel's text under it.
const accordionLook = {
  rootClass: "flex flex_col w_full overflow_hidden rounded_md border_t_1 border_l_1 border_r_1 border_border text_sm",
  itemClass: "flex flex_col border_b_1 border_border",
  headerClass: "flex",
  triggerClass: "flex flex_1 items_center justify_between gap_3 py_2 px_3 font_medium text_muted_foreground",
  triggerHoverClass: "bg_accent text_accent_foreground",
  triggerActiveClass: "bg_muted",
  triggerFocusClass: "bg_accent text_accent_foreground",
  openTriggerClass: "text_foreground",
  disabledTriggerClass: "opacity_50",
  icon: "icons/chevron-down.svg",
  openIcon: "icons/chevron-up.svg",
  iconClass: "size_4 flex_none text_muted_foreground",
  panelClass: "pb_2 px_3",
};

const collapsibleLook = {
  rootClass: "flex flex_col gap_1 w_full text_sm",
  triggerClass: "h_8 px_2 flex items_center gap_2 rounded_md font_medium",
  triggerHoverClass: "bg_accent text_accent_foreground",
  triggerActiveClass: "bg_muted",
  triggerFocusClass: "bg_accent text_accent_foreground",
  disabledTriggerClass: "opacity_50",
  icon: "icons/chevron-right.svg",
  openIcon: "icons/chevron-down.svg",
  iconClass: "size_4 flex_none text_muted_foreground",
  panelClass: "px_2 text_muted_foreground",
};

const items = [
  {
    title: "What does GPUI Base own?",
    content:
      "The behavior of every control: pointer and keyboard interaction, focus, accessibility and state. " +
      "It draws nothing of its own, so a long answer like this one wraps to the width the app gives it.",
  },
  {
    title: "What does the app own?",
    content: "The look: class strings for the root, each part and each state.",
  },
  {
    title: "Can an item be locked?",
    content: "A disabled item ignores the pointer and the keyboard.",
    disabled: true,
  },
];

const single = signal<number[]>([0]);
const several = signal<number[]>([0, 1]);
const details = signal(false);

export function DisclosureDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section title="Accordion" note="One item open at a time; the third is disabled.">
        <View style={{ width: 420 }}>
          <Accordion
            items={items}
            value={single()}
            onChange={(e) => {
              single.set(e.value);
              log("Accordion.change", e);
            }}
            {...accordionLook}
          />
        </View>
      </Section>
      <Section title="Accordion, multiple" note="Any number of items open.">
        <View style={{ width: 420 }}>
          <Accordion
            items={items.slice(0, 2)}
            value={several()}
            multiple
            onChange={(e) => {
              several.set(e.value);
              log("Accordion.change", e);
            }}
            {...accordionLook}
          />
        </View>
      </Section>
      <Section title="Collapsible">
        <View style={{ width: 420, gap: 8 }}>
          <Collapsible
            title="Show details"
            content="Collapsible shows its content under the trigger while open. Click the title, or press Enter or Space while it has focus."
            open={details()}
            onOpenChange={(e) => {
              details.set(e.open);
              log("Collapsible.openChange", e);
            }}
            {...collapsibleLook}
          />
          <Collapsible title="Disabled" content="Never shown." disabled {...collapsibleLook} />
        </View>
      </Section>
    </View>
  );
}
