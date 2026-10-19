import { mount, signal, Text } from "nsh";
import { Button } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const presses = signal(0);
  const inner = Button({ accessibilityLabel: "Inner native action",
    style: { width: 160, height: 56, backgroundColor: "#b91c1c", borderWidth: 2, borderColor: "#111827", borderRadius: 8 },
    children: Text({ children: "Inner action", style: { color: "#ffffff" } }),
    onPress: () => presses.update(n => n + 1),
  });
  return [
    Button({ accessibilityLabel: "Outer native host", style: { width: 320, height: 120, backgroundColor: "#e2e8f0" }, children: inner }),
    Probe({ get snapshot() { return { scene: "composition", presses: presses() }; }, style: { width: 0, height: 0 } }),
  ];
});
