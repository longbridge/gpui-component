import { mount, signal, Text } from "nsh";
import { Button } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const presses = signal(0), secondary = signal(0), disabled = signal(false), focused = signal(null), blurs = signal(0);
  return [
    Button({
      accessibilityLabel: "Run base action",
      style: { width: 240, height: 44, backgroundColor: "#2563eb", borderRadius: 8 },
      children: Text({ children: "Run base action", style: { color: "#ffffff" } }),
      get disabled() { return disabled(); },
      onFocus: () => focused.set("main"),
      onBlur: () => blurs.update(value => value + 1),
      onPress: () => { presses.update(value => value + 1); if (presses() === 3) disabled.set(true); },
    }),
    Button({ accessibilityLabel: "Disabled base action", disabled: true,
      style: { width: 240, height: 44, backgroundColor: "#d1d5db" },
      children: Text({ children: "Disabled action" }), onPress: () => presses.update(value => value + 100),
    }),
    Button({ accessibilityLabel: "Next base action",
      style: { width: 240, height: 44, backgroundColor: "#cbd5e1" },
      children: Text({ children: "Next action" }), onFocus: () => focused.set("next"),
      onPress: () => { secondary.update(value => value + 1); disabled.set(false); },
    }),
    Probe({ get snapshot() { return { ready: true, presses: presses(), secondary: secondary(), disabled: disabled(), focused: focused(), blurs: blurs() }; }, style: { width: 0, height: 0 } }),
  ];
});
