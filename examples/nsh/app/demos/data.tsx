// The gallery's demo of Table and List: rows of data, a selected row, and a
// long list that draws only the rows in view.

import { signal, Text, View } from "nsh";
import { List, Table } from "../../../../crates/nsh/types/gpui-base";
import { log, muted, Section } from "../look";

/** GPUI Kit's table: a muted header, ruled rows, the accent on the selected one. */
const tableLook = {
  rootClass: "w_full rounded_md border_1 border_border overflow_hidden text_sm",
  headerClass: "flex bg_muted text_muted_foreground",
  headClass: "flex_1 px_3 py_2 font_medium",
  rowClass: "flex border_t_1 border_border",
  rowHoverClass: "bg_muted",
  rowActiveClass: "bg_secondary",
  selectedRowClass: "bg_accent text_accent_foreground",
  cellClass: "flex_1 px_3 py_2",
  captionClass: "px_3 py_2 border_t_1 border_border text_xs text_muted_foreground",
};

/** GPUI Kit's list: full-width rows in a bordered box of a fixed height. */
const listLook = {
  rootClass: "w_64 h_64 p_1 rounded_md border_1 border_border text_sm",
  itemClass: "w_full px_2 flex items_center rounded_sm",
  itemHoverClass: "bg_muted",
  itemActiveClass: "bg_secondary",
  selectedItemClass: "bg_accent text_accent_foreground",
};

const columns = [
  { title: "Crate", width: 160 },
  { title: "Status" },
  { title: "Version", width: 96, align: "right" as const },
];
const rows = [
  ["gpui-base", "Stable", "0.4.1"],
  ["gpui-component", "Active", "0.4.1"],
  ["gpui-shell", "Preview", "0.2.8"],
  ["gpui-nsh", "Beta", "0.1.0"],
];

const customers = Array.from({ length: 10_000 }, (_, ix) => `Customer ${String(ix + 1).padStart(5, "0")}`);
const fruit = ["Apple", "Banana", "Cherry", "Grape", "Mango", "Orange", "Peach", "Pear"];

const crate = signal(1);
const customer = signal(-1);
const chosenFruit = signal(2);

export function DataDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Table"
        note="Columns take a fixed width or share the rest, and align their text. Click a row to select it."
      >
        <View style={{ width: 480 }}>
          <Table
            columns={columns}
            rows={rows}
            caption="The crates of GPUI Kit."
            selected={crate()}
            accessibilityLabel="Crates"
            {...tableLook}
            onSelect={(e) => {
              crate.set(e.value);
              log("Table.select", e);
            }}
          />
        </View>
      </Section>
      <Section
        title="List"
        note="10,000 items: only the rows in view are drawn. The list scrolls itself inside the height its rootClass gives it."
      >
        <View style={{ gap: 8 }}>
          <List
            items={customers}
            selected={customer()}
            accessibilityLabel="Customers"
            {...listLook}
            onSelect={(e) => {
              customer.set(e.value);
              log("List.select", e);
            }}
          />
          <Text class={`${muted} text-xs`}>
            {customer() < 0 ? "No customer selected" : customers[customer()]}
          </Text>
        </View>
        <List
          items={fruit}
          itemHeight={24}
          selected={chosenFruit()}
          accessibilityLabel="Fruit"
          {...listLook}
          rootClass="w_40 h_40 p_1 rounded_md border_1 border_border text_sm"
          onSelect={(e) => {
            chosenFruit.set(e.value);
            log("List.select", e);
          }}
        />
      </Section>
    </View>
  );
}
