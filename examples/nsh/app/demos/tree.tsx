// The gallery's demo of Tree: a file tree with the keyboard, its expanded
// folders and selection kept by the app, and a disabled item.

import { signal, Text, View } from "nsh";

import { Button, Tree } from "../../../../crates/nsh/types/gpui-base";
import { button, buttonStates, log, muted, Section } from "../look";

/** GPUI Kit's tree: rounded rows indented per level, the selected row in the accent. */
const treeLook = {
  rootClass: "w_72 h_64 p_1 rounded_md border_1 border_border bg_background",
  focusedClass: "border_ring",
  listClass: "size_full",
  indent: 16,
  expandIcon: "icons/chevron-right.svg",
  collapseIcon: "icons/chevron-down.svg",
  itemClass: "w_full h_7 px_2 flex items_center gap_1 rounded_md text_sm text_foreground",
  itemHoverClass: "bg_muted",
  itemActiveClass: "bg_muted",
  selectedItemClass: "bg_accent text_accent_foreground",
  disabledItemClass: "text_muted_foreground opacity_50",
  disclosureClass: "size_4 flex_none flex items_center justify_center",
  iconClass: "size_3p5 text_muted_foreground",
  labelClass: "truncate",
};

const files = [
  {
    id: "crates",
    label: "crates",
    children: [
      {
        id: "crates/base",
        label: "base",
        children: [
          { id: "crates/base/Cargo.toml", label: "Cargo.toml" },
          {
            id: "crates/base/src",
            label: "src",
            children: [
              { id: "crates/base/src/lib.rs", label: "lib.rs" },
              { id: "crates/base/src/tree.rs", label: "tree.rs" },
            ],
          },
        ],
      },
      {
        id: "crates/nsh",
        label: "nsh",
        children: [
          { id: "crates/nsh/README.md", label: "README.md" },
          { id: "crates/nsh/generated", label: "generated (read-only)", disabled: true },
        ],
      },
    ],
  },
  { id: "docs", label: "docs", children: [{ id: "docs/ARCHITECTURE.md", label: "ARCHITECTURE.md" }] },
  { id: "Cargo.toml", label: "Cargo.toml" },
];

const expanded = signal<string[]>(["crates", "crates/base"]);
const selected = signal("crates/base/Cargo.toml");

export function TreeDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Tree with the keyboard"
        note="Click a folder to expand or collapse it. Tab to the tree: Up and Down move the selection, Right expands and Left collapses the selected folder, and Enter toggles it. The read-only item takes no click, and the arrows pass it by."
      >
        <Tree
          items={files}
          expanded={expanded()}
          selected={selected()}
          {...treeLook}
          onSelect={(e) => {
            // An empty id: the selected item went away with its folder.
            selected.set(e.id);
            log("Tree.select", e);
          }}
          onExpandedChange={(e) => {
            expanded.set(e.expanded);
            log("Tree.expandedChange", e);
          }}
        />
        <View style={{ gap: 8 }}>
          <Text class={`${muted} text-xs`}>{`Selected: ${selected() || "none"}`}</Text>
          <Text class={`${muted} text-xs`}>{`Expanded: ${expanded().join(", ") || "none"}`}</Text>
          <Button
            label="Reveal tree.rs"
            rootClass={button}
            {...buttonStates}
            onClick={() => {
              // Selecting a hidden item expands its folders, which the tree reports.
              selected.set("crates/base/src/tree.rs");
              log("Button.click", { reveal: "crates/base/src/tree.rs" });
            }}
          />
          <Button
            label="Collapse all"
            rootClass={button}
            {...buttonStates}
            onClick={() => {
              expanded.set([]);
              log("Button.click", { expanded: [] });
            }}
          />
        </View>
      </Section>
    </View>
  );
}
