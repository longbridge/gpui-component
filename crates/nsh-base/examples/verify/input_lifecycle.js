import { mount, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const value = signal("abc"), changes = signal(0), focused = signal(false);
  const presentation = signal(false), visible = signal(true), removeNext = signal(false);
  const focuses = signal(0), errors = signal([]), staleCommands = signal(0);
  let input, oldInput;
  const action = (label, onPress) => Button({ accessibilityLabel: label,
    style: { width: 300, height: 36 }, children: Text({ children: label }), onPress });
  return View({ style: { padding: 20, gap: 8 }, children: [
    () => visible() ? Input({ ref: node => input = node, accessibilityLabel: "Lifecycle input",
      get value() { return value(); },
      get placeholder() { return presentation() ? "Updated hint" : "Original hint"; },
      get style() { return { width: 320, height: 44, padding: 8, backgroundColor: presentation() ? "#dcfce7" : "#dbeafe" }; },
      onChange: next => { changes.update(n => n + 1); value.set(next);
        if (removeNext()) { oldInput = input; visible.set(false); }
      },
      onFocus: () => { focused.set(true); focuses.update(n => n + 1); },
      onBlur: () => focused.set(false),
      onError: error => errors.update(all => [...all, error]),
    }) : View({ style: { width: 320, height: 44 }, children: Text({ children: "Input removed" }) }),
    action("Focus lifecycle input", () => input.command("focus")),
    action("Update presentation", () => { presentation.set(true); input.command("focus"); }),
    action("Remove on next edit", () => { removeNext.set(true); input.command("focus"); }),
    action("Command removed input", () => { oldInput.command("focus"); oldInput.command("selectAll"); staleCommands.update(n => n + 1); }),
    action("Restore lifecycle input", () => { removeNext.set(false); visible.set(true); }),
    Probe({ get snapshot() { return { scene: "input-lifecycle", value: value(), changes: changes(),
      presentation: presentation(), visible: visible(), focused: focused(), focuses: focuses(),
      staleCommands: staleCommands(), errors: errors() }; }, style: { width: 0, height: 0 } }),
  ] });
});
