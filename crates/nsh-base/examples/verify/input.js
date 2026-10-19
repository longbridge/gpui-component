import { mount, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const value = signal("abc"), changes = signal(0), focused = signal(false);
  const focuses = signal(0), blurs = signal(0), errors = signal([]);
  const allow = signal(true), readonly = signal(false), disabled = signal(false);
  const submits = signal([]);
  let input;
  const action = (label, onPress) => Button({ accessibilityLabel: label,
    style: { width: 220, height: 32 }, children: Text({ children: label }), onPress });
  return View({ style: { padding: 24, gap: 12 }, children: [
    Input({ ref: node => input = node, accessibilityLabel: "Editable task",
      get value() { return value(); }, placeholder: "Task name",
      get readonly() { return readonly(); }, get disabled() { return disabled(); },
      style: { width: 320, height: 48, padding: 12, fontSize: 20,
        color: "#102030", backgroundColor: "#e8f0fc" },
      onChange: next => { changes.update(n => n + 1); if (allow()) value.set(next); },
      onSubmit: event => submits.update(all => [...all, event]),
      onFocus: () => { focused.set(true); focuses.update(n => n + 1); },
      onBlur: () => { focused.set(false); blurs.update(n => n + 1); },
      onError: message => errors.update(all => [...all, message]),
    }),
    Button({ accessibilityLabel: "Focus task", style: { width: 220, height: 36 },
      children: Text({ children: "Focus task" }), onPress: () => input.command("focus") }),
    action("Reject edits", () => { allow.set(false); input.command("focus"); }),
    action("Accept edits", () => { allow.set(true); input.command("focus"); }),
    action("Readonly task", () => { readonly.set(true); input.command("focus"); }),
    action("Editable task again", () => { readonly.set(false); input.command("focus"); }),
    action("Disable task", () => disabled.set(true)),
    action("Enable task", () => { disabled.set(false); input.command("focus"); }),
    action("Select task text", () => { input.command("focus"); input.command("selectAll"); }),
    Probe({ get snapshot() { return { scene: "input", value: value(), changes: changes(),
      focused: focused(), focuses: focuses(), blurs: blurs(), errors: errors(),
      allow: allow(), readonly: readonly(), disabled: disabled(), submits: submits() }; },
      style: { width: 0, height: 0 } }),
  ] });
});
