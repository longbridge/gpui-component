import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
const Button = nativeComponent("gpui-component.Button", { props: ["label"], events: ["press"] });
const Dialog = nativeComponent("gpui-component.Dialog", {
  props: ["open", "title", "closeButton"], events: ["openChange", "confirm", "cancel"],
});
const Footer = nativeComponent("gpui-component.DialogFooter", {});
const Action = nativeComponent("gpui-component.DialogAction", { props: ["text"] });
const Close = nativeComponent("gpui-component.DialogClose", { props: ["text"] });
const Slot = nativeComponent("__nsh.Slot", { props: ["__nshSlot"] });
const Probe = nativeComponent("verify.probe", { props: ["snapshot"], events: ["command"] });
mount(() => {
  const open = signal(false), confirms = signal(0), cancels = signal(0), closes = signal(0), headerClicks = signal(0);
  const action = Action({ text: "Save text part", style: { width: 150, height: 36 } });
  const cancel = Close({ text: "Cancel text part", style: { width: 150, height: 36 } });
  const footer = Footer({ style: { width: 320, height: 36 } });
  insert(footer, () => [action, cancel]);
  const slot = Slot({ __nshSlot: "footer", style: { width: 320, height: 36 } });
  insert(slot, () => footer);
  const dialog = Dialog({
    get open() { return open(); }, title: "Keyboard text parts", closeButton: false,
    onConfirm: () => confirms.update(n => n + 1),
    onCancel: () => cancels.update(n => n + 1),
    onOpenChange: value => { open.set(value); if (!value) closes.update(n => n + 1); },
  });
  const header = Slot({ __nshSlot: "header", style: { width: 320, height: 36 } });
  insert(header, () => Button({ label: "Named header action", style: { width: 180, height: 36 }, onPress: () => headerClicks.update(n => n + 1) }));
  const title = Slot({ __nshSlot: "title", style: { width: 320, height: 36 } });
  insert(title, () => Button({ label: "Named title content", style: { width: 180, height: 36 } }));
  const content = Slot({ __nshSlot: "content", style: { width: 320, height: 36 } });
  insert(content, () => Button({ label: "Named body content", style: { width: 180, height: 36 } }));
  insert(dialog, () => [header, title, content, slot]);
  return [
    Button({ label: "Increment", style: { width: 180, height: 36 }, onPress: () => open.set(true) }),
    dialog,
    Probe({
      get snapshot() { return { fixture: "dialog_parts", open: open(), confirms: confirms(), cancels: cancels(), closes: closes(), headerClicks: headerClicks() }; },
      onCommand: target => {
        if (target === "open") open.set(true);
        else (target === "action" ? action : cancel).command("focus", null);
      },
    }),
  ];
});
