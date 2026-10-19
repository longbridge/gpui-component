// The gallery's demo of Dialog, AlertDialog and Sheet. Each is its trigger
// button here; GPUI Base draws the modal over the whole window, traps the
// focus in it and dismisses it, and the app answers `openChange` by setting
// `open`.

import { signal, View } from "nsh";
import { AlertDialog, Dialog, Sheet, ToggleGroup } from "../../../../crates/nsh/types/gpui-base";
import { button, buttonStates, primary, ring, segmented, segment, log, Section } from "../look";

// GPUI Kit's modal look: the window dims behind a bordered surface.
const backdrop = "size_full bg_foreground opacity_5";
const surface = "flex flex_col gap_4 bg_background border_1 border_border shadow_lg";
const title = "text_base font_semibold";
const description = "text_sm text_muted_foreground";
const footer = "flex justify_end gap_2";
const primaryStates = { actionHoverClass: "opacity_90", actionActiveClass: "opacity_80", actionFocusClass: ring };
const cancelStates = { cancelHoverClass: "bg_accent", cancelActiveClass: "bg_muted", cancelFocusClass: ring };
/** A small ghost icon button, as GPUI Kit's close buttons. */
const close = {
  closeIcon: "icons/close.svg",
  closeLabel: "Close",
  closeClass: "size_6 flex items_center justify_center rounded_md text_muted_foreground",
  closeHoverClass: "bg_accent text_accent_foreground",
  closeActiveClass: "bg_muted",
  closeFocusClass: ring,
  closeIconClass: "size_4",
};

const dialogOpen = signal(false);
const alertOpen = signal(false);
const sheetOpen = signal(false);
const side = signal(0);

/** Where the sheet's surface sits, by the side chosen. */
const sides = [
  "absolute top_0 right_0 h_full w_80 border_l_1",
  "absolute top_0 left_0 h_full w_80 border_r_1",
  "absolute top_0 left_0 w_full h_48 border_b_1",
  "absolute bottom_0 left_0 w_full h_48 border_t_1",
];

export function OverlayDemo() {
  return (
    <View style={{ gap: 20 }}>
      <Section
        title="Dialog"
        note="Escape, Cancel, the close button or a press outside cancels; Enter or Save confirms. Tab stays inside."
      >
        <Dialog
          label="Edit profile"
          open={dialogOpen()}
          onOpenChange={(e) => {
            dialogOpen.set(e.open);
            log("Dialog.openChange", e);
          }}
          onAction={(e) => log("Dialog.action", e)}
          onCancel={(e) => log("Dialog.cancel", e)}
          title="Edit profile"
          description="Make changes to your profile here. Save when you are done."
          actionLabel="Save changes"
          cancelLabel="Cancel"
          {...close}
          rootClass={button}
          {...buttonStates}
          backdropClass={backdrop}
          popupClass={`w_112 p_4 rounded_lg ${surface}`}
          headerClass="flex items_start justify_between gap_2"
          titleClass={title}
          descriptionClass={description}
          footerClass={footer}
          actionClass={primary}
          {...primaryStates}
          cancelClass={button}
          {...cancelStates}
        />
        <Dialog
          label="Disabled"
          disabled
          rootClass={button}
          disabledClass="opacity_50"
        />
      </Section>
      <Section title="AlertDialog" note="A press outside leaves it open: answer with Delete, Cancel, Enter or Escape.">
        <AlertDialog
          label="Delete file"
          open={alertOpen()}
          onOpenChange={(e) => {
            alertOpen.set(e.open);
            log("AlertDialog.openChange", e);
          }}
          onAction={(e) => log("AlertDialog.action", e)}
          onCancel={(e) => log("AlertDialog.cancel", e)}
          title="Delete “report.pdf”?"
          description="The file is deleted from this device. This cannot be undone."
          actionLabel="Delete"
          cancelLabel="Cancel"
          rootClass="h_8 px_3 flex items_center rounded_md bg_destructive text_destructive_foreground text_sm"
          hoverClass="opacity_90"
          activeClass="opacity_80"
          focusClass={ring}
          backdropClass={backdrop}
          popupClass={`w_96 p_4 rounded_lg ${surface}`}
          titleClass={title}
          descriptionClass={description}
          footerClass={footer}
          actionClass="h_8 px_3 flex items_center rounded_md bg_destructive text_destructive_foreground text_sm"
          {...primaryStates}
          cancelClass={button}
          {...cancelStates}
        />
      </Section>
      <Section title="Sheet" note="The surface's class places it at a side of the window.">
        <ToggleGroup
          items={["Right", "Left", "Top", "Bottom"]}
          value={side()}
          onChange={(e) => side.set(e.value)}
          rootClass={segmented}
          toggleClass={segment}
          toggleHoverClass="text_foreground"
          toggleFocusClass={ring}
          pressedClass="bg_background text_foreground"
        />
        <Sheet
          label="Open settings"
          open={sheetOpen()}
          onOpenChange={(e) => {
            sheetOpen.set(e.open);
            log("Sheet.openChange", e);
          }}
          title="Settings"
          description="Choose how the app looks and behaves. Escape, the close button or a press outside closes it."
          {...close}
          rootClass={button}
          {...buttonStates}
          backdropClass={`absolute inset_0 ${backdrop}`}
          popupClass={`${sides[side()]} flex flex_col gap_2 pb_4 bg_background border_border shadow_xl`}
          headerClass="flex items_center justify_between pl_4 pr_3 py_2"
          titleClass="font_semibold"
          descriptionClass={`px_4 ${description}`}
        />
      </Section>
    </View>
  );
}
