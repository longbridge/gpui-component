// The gallery's demo of ToastStack. The app keeps its toasts as a list and
// adds to it; GPUI Base shows each new entry at a corner of the window, times
// it, stacks it and lets it leave, and the app answers `dismiss` by removing
// the entry.

import { signal, View } from "nsh";
import { Button, ToastStack, ToggleGroup } from "../../../../crates/nsh/types/gpui-base";
import { button, buttonStates, primary, ring, segmented, segment, log, Section } from "../look";

type Toast = { id: string; title: string; description?: string; duration?: number };

// GPUI Kit's notification look: a bordered surface with a shadow, a semibold
// title over the message, and a small ghost close button in its corner.
const toastLook = {
  rootClass: "w_96 p_4",
  toastClass: "relative flex flex_col gap_1 py_3p5 px_4 rounded_lg border_1 border_border bg_background shadow_lg",
  toastHoverClass: "border_ring",
  toastEndingClass: "opacity_0",
  titleClass: "pr_6 text_sm font_semibold",
  descriptionClass: "pr_6 text_sm text_muted_foreground",
  closeIcon: "icons/close.svg",
  closeLabel: "Close",
  closeClass: "absolute top_1 right_1 size_6 flex items_center justify_center rounded_md text_muted_foreground",
  closeHoverClass: "bg_accent text_accent_foreground",
  closeActiveClass: "bg_muted",
  closeFocusClass: ring,
  closeIconClass: "size_4",
};

const placements = ["topRight", "topLeft", "topCenter", "bottomRight", "bottomLeft", "bottomCenter"] as const;
const placementNames = ["Top right", "Top left", "Top center", "Bottom right", "Bottom left", "Bottom center"];

const toasts = signal<Toast[]>([]);
const placement = signal(0);
let next = 0;

function show(toast: Omit<Toast, "id">) {
  next += 1;
  toasts.update((list) => [...list, { id: `toast-${next}`, ...toast }]);
}

export function ToastDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="ToastStack"
        note="Hover the stack, or Tab into it, to expand it and pause the timers. The close button or the time running out dismisses a toast."
      >
        <Button
          label="Saved"
          rootClass={primary}
          hoverClass="opacity_90"
          activeClass="opacity_80"
          focusClass={ring}
          onClick={() => show({ title: "Saved", description: "Your changes were saved.", duration: 5000 })}
        />
        <Button
          label="Short"
          rootClass={button}
          {...buttonStates}
          onClick={() => show({ title: "Copied to the clipboard", duration: 2000 })}
        />
        <Button
          label="Until dismissed"
          rootClass={button}
          {...buttonStates}
          onClick={() =>
            show({
              title: "Update available",
              description: "Restart the app to finish installing version 2.0.",
            })
          }
        />
        <Button label="Clear all" rootClass={button} {...buttonStates} onClick={() => toasts.set([])} />
        <ToastStack
          toasts={toasts()}
          placement={placements[placement()]}
          limit={3}
          {...toastLook}
          onDismiss={(e) => {
            toasts.update((list) => list.filter((toast) => toast.id !== e.id));
            log("ToastStack.dismiss", e);
          }}
        />
      </Section>
      <Section title="Placement">
        <ToggleGroup
          items={placementNames}
          value={placement()}
          onChange={(e) => placement.set(e.value)}
          rootClass={segmented}
          toggleClass={segment}
          toggleHoverClass="text_foreground"
          toggleFocusClass={ring}
          pressedClass="bg_background text_foreground"
        />
      </Section>
    </View>
  );
}
