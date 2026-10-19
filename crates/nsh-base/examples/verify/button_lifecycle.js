import { mount, signal, Text } from "nsh";
import { Button } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const shown = signal(true), armed = signal(false), disabled = signal(false);
  const presses = signal(0), focused = signal(false), errors = signal([]);
  let transient;
  const control = (label, onPress) => Button({ accessibilityLabel: label,
    style: { width: 280, height: 36, backgroundColor: "#e2e8f0" },
    children: Text({ children: label }), onPress,
  });
  return [
    () => shown() ? Button({ ref: node => transient = node, accessibilityLabel: "Transient action",
      get disabled() { return disabled(); }, style: { width: 280, height: 44, backgroundColor: "#dbeafe" },
      children: Text({ children: "Transient action" }),
      onFocus: () => focused.set(true), onBlur: () => focused.set(false),
      onError: message => errors.update(all => [...all, message]),
      onPress: () => { presses.update(n => n + 1); if (armed()) shown.set(false); },
    }) : [],
    control("Focus transient", () => transient?.command("focus")),
    control("Corrupt button props", () => disabled.set("invalid boolean")),
    control("Restore button props", () => disabled.set(false)),
    control("Arm focused unmount", () => { armed.set(true); transient?.command("focus"); }),
    control("Remount transient", () => { armed.set(false); shown.set(true); }),
    Probe({ get snapshot() { return { scene: "button-lifecycle", shown: shown(), presses: presses(), focused: focused(), errors: errors() }; }, style: { width: 0, height: 0 } }),
  ];
});
