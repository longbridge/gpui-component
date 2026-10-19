import { mount, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const values = signal({ zero: "zero", negative: "negative", excluded: "excluded", three: "three", two: "two" });
  const focused = signal(""), errors = signal([]), history = signal([]);
  const refs = {};
  const field = (name, tabIndex, tabStop = true) => Input({
    ref: node => refs[name] = node, accessibilityLabel: `Navigation ${name}`,
    tabIndex, tabStop, get value() { return values()[name]; },
    style: { width: 300, height: 44, padding: 8 },
    onChange: value => values.update(all => ({ ...all, [name]: value })),
    onFocus: () => { focused.set(name); history.update(all => [...all, name]); },
    onBlur: () => { if (focused() === name) focused.set(""); },
    onError: message => errors.update(all => [...all, message]),
  });
  const focus = name => Button({ accessibilityLabel: `Focus navigation ${name}`,
    style: { width: 260, height: 32 }, children: Text({ children: `Focus ${name}` }),
    onPress: () => refs[name].command("focus") });
  return View({ style: { padding: 20, gap: 8 }, children: [
    field("zero", 0), field("negative", -1), field("excluded", 0, false),
    field("three", 3), field("two", 2),
    focus("negative"), focus("excluded"), focus("two"),
    Probe({ get snapshot() { return { scene: "input-navigation", values: values(), focused: focused(), history: history(), errors: errors() }; },
      style: { width: 0, height: 0 } }),
  ] });
});
