import { mount, signal } from "nsh";
import { Button, Checkbox, Radio, Switch } from "nsh/ui/gpui";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"], events: ["command"] });
mount(() => {
  const events = signal(0), commanded = signal("");
  const refs = {};
  const props = (label, index, stop = true) => ({ accessibilityLabel: label, label,
    tabIndex: index, tabStop: stop, ref: node => refs[label] = node,
    style: { width: 320, height: 36 } });
  return [
    Button({ ...props("Increment", 2), onPress: () => events.update(n => n + 1) }),
    Checkbox({ ...props("Focus checkbox three", 3), checked: false, onChange: () => events.update(n => n + 1) }),
    Radio({ ...props("Focus radio zero", 0), checked: false, onChange: () => events.update(n => n + 1) }),
    Switch({ ...props("Focus switch zero", 0), checked: false, onChange: () => events.update(n => n + 1) }),
    Button({ ...props("Focus button negative", -1), onPress: () => events.update(n => n + 1) }),
    Checkbox({ ...props("Focus checkbox excluded", 0, false), checked: false, onChange: () => events.update(n => n + 1) }),
    Radio({ ...props("Focus radio negative", -1), checked: false, onChange: () => events.update(n => n + 1) }),
    Switch({ ...props("Focus switch excluded", 0, false), checked: false, onChange: () => events.update(n => n + 1) }),
    Probe({ get snapshot() { return { fixture: "component-focus", events: events(), commanded: commanded() }; },
      onCommand: label => { refs[label].command("focus"); commanded.set(label); }, style: { width: 0, height: 0 } }),
  ];
});
