import { mount, signal, Text } from "nsh";
import { Button } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const focused = signal(null), presses = signal([]);
  const action = (label, extra) => Button({
    accessibilityLabel: label, style: { width: 240, height: 40, backgroundColor: "#dbeafe" },
    children: Text({ children: label }), onFocus: () => focused.set(label),
    onPress: () => presses.update(all => [...all, label]), ...extra,
  });
  return [
    action("Index three", { tabIndex: 3 }),
    action("Disabled index one", { tabIndex: 1, disabled: true }),
    action("Index two", { tabIndex: 2 }),
    action("Skipped action", { tabStop: false }),
    action("Negative action", { tabIndex: -1 }),
    action("Index zero", {}),
    Probe({ get snapshot() { return { scene: "button-focus", focused: focused(), presses: presses() }; }, style: { width: 0, height: 0 } }),
  ];
});
