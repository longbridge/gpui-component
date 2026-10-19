import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { Dialog, Sheet } from "nsh/ui/gpui";

const Button = nativeComponent("gpui-component.Button", { props: ["label"], events: ["press"] });
const Notification = nativeComponent("gpui-component.Notification", {
  props: ["open", "message", "autohide"], events: ["openChange", "error"],
});
const Probe = nativeComponent("verify.probe", { props: ["snapshot"], events: ["command"] });

mount(() => {
  const count = signal(0);
  const mounted = signal(true);
  const firstSheetMounted = signal(true);
  const firstNoteMounted = signal(true);
  const dialog = signal(false);
  const sheet = signal(false);
  const replacement = signal(false);
  const note1 = signal(false);
  const note2 = signal(false);
  const title = signal("NSH dialog");
  const confirms = signal(0);
  const cancels = signal(0);
  const dialogCloses = signal(0);
  const sheetCloses = signal(0);
  const replacementCloses = signal(0);
  const noteCloses = signal(0);
  const error = signal(null);
  let dialogButton, sheetButton;
  const box = { width: 220, height: 36 };
  return [
    Button({ label: "Increment", style: box, onPress: () => count.update((value) => value + 1) }),
    Button({ ref: value => dialogButton = value, label: "Open dialog…", style: box, onPress: () => { dialogButton.command("focus", null); dialog.set(true); } }),
    Button({ ref: value => sheetButton = value, label: "Open sheet…", style: box, onPress: () => { sheetButton.command("focus", null); sheet.set(true); } }),
    Button({ label: "Show notifications", style: box, onPress: () => { note1.set(true); note2.set(true); } }),
    () => mounted() ? [
      Dialog({
        get open() { return dialog(); }, get title() { return title(); },
        description: "Window-level content", confirmLabel: "Apply", showCancel: true,
        onConfirm: () => confirms.update((value) => value + 1),
        onCancel: () => cancels.update((value) => value + 1),
        onOpenChange: (open) => { dialog.set(open); dialogCloses.update((value) => value + 1); },
        onError: (value) => error.set(value),
      }),
      () => firstSheetMounted() ? Sheet({
        get open() { return sheet(); }, title: "First sheet", description: "First content", size: 224,
        onOpenChange: (open) => { sheet.set(open); sheetCloses.update((value) => value + 1); },
        onError: (value) => error.set(value),
      }) : [],
      Sheet({
        get open() { return replacement(); }, title: "Replacement sheet", description: "Replacement content", size: 260,
        onOpenChange: (open) => { replacement.set(open); replacementCloses.update((value) => value + 1); },
        onError: (value) => error.set(value),
      }),
      () => firstNoteMounted() ? Notification({
        get open() { return note1(); }, message: "First notification", autohide: false,
        onOpenChange: (open) => { note1.set(open); noteCloses.update((value) => value + 1); },
        onError: (value) => error.set(value),
      }) : [],
      Notification({
        get open() { return note2(); }, message: "Second notification", autohide: false,
        onOpenChange: (open) => { note2.set(open); noteCloses.update((value) => value + 1); },
        onError: (value) => error.set(value),
      }),
    ] : [],
    Probe({
      get snapshot() { return {
        fixture: "overlays", count: count(), mounted: mounted(), dialog: dialog(),
        sheet: sheet(), replacement: replacement(), note1: note1(), note2: note2(),
        firstSheetMounted: firstSheetMounted(), firstNoteMounted: firstNoteMounted(),
        title: title(), confirms: confirms(), cancels: cancels(), dialogCloses: dialogCloses(),
        sheetCloses: sheetCloses(), replacementCloses: replacementCloses(), noteCloses: noteCloses(), error: error(),
      }; },
      onCommand: (command) => {
        switch (command) {
          case "close-dialog": dialog.set(false); break;
          case "change-title": title.set("Updated dialog"); break;
          case "replace-sheet": replacement.set(true); break;
          case "unmount-first-sheet": firstSheetMounted.set(false); break;
          case "unmount-first-note": firstNoteMounted.set(false); note1.set(false); break;
          case "unmount-overlays":
            mounted.set(false); dialog.set(false); sheet.set(false); replacement.set(false); note1.set(false); note2.set(false);
            break;
          case "mount-overlays": mounted.set(true); break;
        }
      }, style: { width: 0, height: 0 },
    }),
  ];
});
