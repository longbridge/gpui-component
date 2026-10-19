import { mount, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const changed = signal(false), value = signal("NSH Kit"), errors = signal([]);
  return View({ get style() { return { padding: 24, gap: 16,
    color: changed() ? "#8a1428" : "#102030", fontSize: changed() ? 28 : 20 }; }, children: [
    Input({ accessibilityLabel: "Styled native input", get value() { return value(); },
      onChange: text => value.set(text), onError: error => errors.update(all => [...all, error]),
      get style() { return { width: 360, height: 76, padding: changed() ? 20 : 12,
        backgroundColor: "#e8f0fc" }; },
    }),
    Button({ accessibilityLabel: "Change inherited input style",
      style: { width: 320, height: 44 }, children: Text({ children: "Change inherited style" }),
      onPress: () => changed.set(true),
    }),
    Probe({ get snapshot() { return { scene: "input-style", changed: changed(), value: value(), errors: errors() }; },
      style: { width: 0, height: 0 } }),
  ] });
});
