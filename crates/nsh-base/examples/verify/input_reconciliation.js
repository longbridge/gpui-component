import { mount, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const value = signal("abc"), allow = signal(true), changes = signal(0);
  const resets = signal(0), focused = signal(false), errors = signal([]);
  const disabled = signal(false), placeholder = signal("");
  const invalidNext = signal(false);
  let acceptedBefore = "";
  let input;
  const action = (label, onPress) => Button({ accessibilityLabel: label,
    style: { width: 280, height: 36 }, children: Text({ children: label }), onPress });
  return View({ style: { padding: 24, gap: 12 }, children: [
    Input({ ref: node => input = node, accessibilityLabel: "Reconciled task",
      get value() { return value(); },
      get disabled() { return disabled(); }, get placeholder() { return placeholder(); },
      style: { width: 360, height: 48, padding: 12, fontSize: 20 },
      onChange: next => {
        changes.update(n => n + 1);
        if (invalidNext()) {
          acceptedBefore = value(); invalidNext.set(false);
          // Deliberate public JS contract violations: reject the complete batch,
          // including the valid disabled=true sibling, and report one error.
          value.set(42); placeholder.set(7); disabled.set(true);
        } else if (allow()) value.set(next);
      },
      onFocus: () => focused.set(true), onBlur: () => focused.set(false),
      onError: message => errors.update(all => [...all, message]),
    }),
    action("Reset task externally", () => {
      value.set("wxyz"); allow.set(false); resets.update(n => n + 1); input.command("focus");
    }),
    action("Accept reconciled edits", () => { allow.set(true); input.command("focus"); }),
    action("Reset Unicode task externally", () => {
      value.set("甲乙丙丁"); allow.set(false); resets.update(n => n + 1); input.command("focus");
    }),
    action("Reject invalid batch on next edit", () => {
      invalidNext.set(true); allow.set(false); input.command("focus");
    }),
    action("Recover valid task props", () => {
      value.set(acceptedBefore); placeholder.set(""); disabled.set(false);
      allow.set(true); input.command("focus");
    }),
    Probe({ get snapshot() { return { scene: "input-reconciliation", value: value(),
      allow: allow(), disabled: disabled(), changes: changes(), resets: resets(), focused: focused(), errors: errors() }; },
      style: { width: 0, height: 0 } }),
  ] });
});
