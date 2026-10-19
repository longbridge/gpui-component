// The gallery's demo of Toolbar: icon buttons and toggles in groups, roving
// focus with Left and Right, and a disabled toolbar.

import { signal, Text, View } from "nsh";

import { Toolbar } from "../../../../crates/nsh/types/gpui-base";
import { accent, log, muted, ring, Section } from "../look";

/** GPUI Kit's toolbar items: ghost, compact buttons that show the accent on hover and when pressed. */
const toolbarLook = {
  rootClass: "flex items_center gap_1 p_1 rounded_md border_1 border_border bg_background",
  groupClass: "gap_0p5",
  itemClass: "h_7 px_1p5 flex items_center justify_center gap_1 rounded_md text_sm text_foreground",
  itemHoverClass: accent,
  itemActiveClass: "bg_muted",
  itemFocusClass: ring,
  pressedClass: accent,
  disabledClass: "opacity_50",
  separatorClass: "w_px h_5 mx_1 bg_border",
  iconClass: "size_4 text_foreground",
};

const marks = signal({ bold: true, italic: false, underline: false });
const align = signal("left");
const history = signal(0);

export function ToolbarDemo() {
  const m = marks();
  const items = [
    { id: "undo", icon: "icons/undo.svg", accessibilityLabel: "Undo", group: "History" },
    { id: "redo", icon: "icons/redo.svg", accessibilityLabel: "Redo", group: "History" },
    { separator: true as const },
    { id: "bold", icon: "icons/bold.svg", accessibilityLabel: "Bold", pressed: m.bold, group: "Format" },
    { id: "italic", icon: "icons/italic.svg", accessibilityLabel: "Italic", pressed: m.italic, group: "Format" },
    {
      id: "underline",
      icon: "icons/underline.svg",
      accessibilityLabel: "Underline",
      pressed: m.underline,
      group: "Format",
    },
    { separator: true as const },
    ...[
      ["left", "start"],
      ["center", "center"],
      ["right", "end"],
    ].map(([side, icon]) => ({
      id: side,
      icon: `icons/text-align-${icon}.svg`,
      accessibilityLabel: `Align ${side}`,
      pressed: align() === side,
      group: "Alignment",
    })),
    { separator: true as const },
    { id: "share", icon: "icons/share.svg", label: "Share" },
    { id: "publish", label: "Publish", disabled: true },
  ];
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Toolbar"
        note="Buttons send click, items with `pressed` are toggles sending change. Tab into it, then Left and Right move between the items."
      >
        <Toolbar
          items={items}
          onClick={(e) => {
            if (e.id === "undo") history.update((n) => n - 1);
            if (e.id === "redo") history.update((n) => n + 1);
            log("Toolbar.click", e);
          }}
          onChange={(e) => {
            if (e.id === "bold" || e.id === "italic" || e.id === "underline") {
              marks.set({ ...marks(), [e.id]: e.pressed });
            } else if (e.pressed) {
              // The alignments are exclusive: the app keeps one pressed.
              align.set(e.id);
            }
            log("Toolbar.change", e);
          }}
          {...toolbarLook}
        />
        <Text class={`${muted} text-xs`}>{`History ${history()}, align ${align()}`}</Text>
      </Section>
      <Section title="Disabled toolbar">
        <Toolbar items={items.slice(0, 6)} disabled {...toolbarLook} />
      </Section>
    </View>
  );
}
