// The gallery's demo of Select and Combobox: keyboard, typeahead, filtering and dismissal.

import { signal, View } from "nsh";
import { Combobox, Select } from "../../../../crates/nsh/types/gpui-base";
import { log, option, popup, ring, Section, trigger, triggerStates } from "../look";

// GPUI Kit's list look: the highlighted row (keyboard or pointer) takes the
// accent, and the chosen value is set in bold, so the highlight moving off it
// never hides which one is chosen.
const listLook = {
  itemClass: option,
  itemHoverClass: "bg_accent text_accent_foreground",
  itemActiveClass: "bg_accent text_accent_foreground",
  highlightedItemClass: "bg_accent text_accent_foreground",
  selectedItemClass: "font_semibold",
};

const countries = [
  "Argentina",
  "Australia",
  "Austria",
  "Belgium",
  "Brazil",
  "Canada",
  "Chile",
  "China",
  "Denmark",
  "Egypt",
  "Finland",
  "France",
  "Germany",
  "Greece",
  "Hungary",
  "Iceland",
  "India",
  "Indonesia",
  "Ireland",
  "Italy",
  "Japan",
  "Kenya",
  "Malaysia",
  "Mexico",
  "Netherlands",
  "New Zealand",
  "Norway",
  "Peru",
  "Poland",
  "Portugal",
  "Singapore",
  "South Korea",
  "Spain",
  "Sweden",
  "Switzerland",
  "Thailand",
  "Turkey",
  "United Kingdom",
  "United States",
  "Vietnam",
];
const country = signal("Japan");
const countryOpen = signal(false);

const sizes = ["Small", "Medium", "Large"];
const size = signal("");
const sizeOpen = signal(false);

const languages = [
  "C",
  "C++",
  "C#",
  "Go",
  "Haskell",
  "Java",
  "JavaScript",
  "Kotlin",
  "Python",
  "Ruby",
  "Rust",
  "Swift",
  "TypeScript",
  "Zig",
];
const language = signal("Rust");
const languageQuery = signal("");
const languageOpen = signal(false);

export function SelectDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Select: keyboard and typeahead"
        note="Tab to it, then type a country's first letters to open on it (repeat a letter to step through the ones starting with it). Up, Down, Home and End move the highlight, Enter chooses, Escape or a click outside closes."
      >
        <Select
          items={countries}
          value={country()}
          open={countryOpen()}
          onOpenChange={(e) => {
            countryOpen.set(e.open);
            log("Select.openChange", e);
          }}
          onChange={(e) => {
            country.set(e.value);
            log("Select.change", e);
          }}
          accessibilityLabel="Country"
          rootClass="w_56"
          triggerClass={trigger}
          {...triggerStates}
          triggerActiveClass="bg_muted"
          contentClass={`w_56 max_h_64 ${popup}`}
          {...listLook}
        />
      </Section>
      <Section title="Select: placeholder and disabled" note="The second one takes no input and is no Tab stop.">
        <Select
          items={sizes}
          value={size()}
          placeholder="Choose a size"
          open={sizeOpen()}
          onOpenChange={(e) => {
            sizeOpen.set(e.open);
            log("Select.openChange", e);
          }}
          onChange={(e) => {
            size.set(e.value);
            log("Select.change", e);
          }}
          accessibilityLabel="Size"
          rootClass="w_40"
          triggerClass={trigger}
          {...triggerStates}
          triggerActiveClass="bg_muted"
          placeholderClass="text_muted_foreground"
          contentClass={`w_40 ${popup}`}
          {...listLook}
        />
        <Select
          items={sizes}
          value="Medium"
          disabled
          accessibilityLabel="Size"
          rootClass="w_40"
          triggerClass={trigger}
          disabledClass="opacity_50"
        />
      </Section>
      <Section
        title="Combobox: filtering"
        note="Opening focuses the search field, which keeps the focus: type to filter, Up and Down move the highlight through the matches, Enter chooses."
      >
        <Combobox
          items={languages}
          value={language()}
          placeholder="Choose a language"
          query={languageQuery()}
          queryPlaceholder="Search…"
          open={languageOpen()}
          onOpenChange={(e) => {
            languageOpen.set(e.open);
            if (!e.open) languageQuery.set("");
            log("Combobox.openChange", e);
          }}
          onQueryChange={(e) => {
            languageQuery.set(e.query);
            log("Combobox.queryChange", e);
          }}
          onChange={(e) => {
            language.set(e.value);
            log("Combobox.change", e);
          }}
          rootClass="w_56"
          triggerClass={trigger}
          {...triggerStates}
          triggerActiveClass="bg_muted"
          placeholderClass="text_muted_foreground"
          contentClass={`w_56 ${popup}`}
          inputClass="w_full h_8 px_2 mb_1 flex items_center rounded_md border_1 border_input text_sm"
          inputFocusedClass={ring}
          listClass="max_h_48"
          {...listLook}
        />
      </Section>
    </View>
  );
}
