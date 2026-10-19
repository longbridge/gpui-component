import { mount, signal, Text, View } from "nsh";
import { Button, Checkbox } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const checked = signal(false), mixed = signal(true), allow = signal(true), disabled = signal(false);
  const changes = signal(0), requested = signal(null), focused = signal(false), errors = signal([]);
  const focuses = signal(0), blurs = signal(0);
  const visible = signal(true), removeOnChange = signal(false);
  let checkbox;
  const action = (label, handler, tabIndex = 0) => Button({ accessibilityLabel: label, tabIndex,
    style: { width: 280, height: 36, backgroundColor: "#e2e8f0" },
    children: Text({ children: label }), onPress: handler,
  });
  return [
    () => visible() ? Checkbox({ ref: node => checkbox = node, accessibilityLabel: "Controlled priority",
      get checked() { return checked(); }, get indeterminate() { return mixed(); },
      get disabled() { return disabled(); },
      style: { width: 280, height: 44, backgroundColor: "#dbeafe" },
      children: View({
        get style() { return { width: 280, height: 44, backgroundColor: mixed() ? "#fef3c7" : checked() ? "#dcfce7" : "#fee2e2" }; },
        children: Text({ get children() { return mixed() ? "Mixed priority" : checked() ? "Priority on" : "Priority off"; } }),
      }),
      onFocus: () => { focused.set(true); focuses.update(n => n + 1); },
      onBlur: () => { focused.set(false); blurs.update(n => n + 1); },
      onChange: value => { changes.update(n => n + 1); requested.set(value);
        if (allow()) { checked.set(value); mixed.set(false); }
        if (removeOnChange()) visible.set(false);
      }, onError: message => errors.update(all => [...all, message]),
    }) : View({ style: { width: 280, height: 44 }, children: Text({ children: "Removed priority" }) }),
    action("Veto mixed changes", () => { checked.set(true); mixed.set(true); allow.set(false); checkbox.command("focus"); }),
    action("Accept checkbox changes", () => { allow.set(true); checkbox.command("focus"); }),
    action("Disable checkbox", () => disabled.set(true)),
    action("Enable checkbox", () => disabled.set(false)),
    action("Remove on next change", () => { removeOnChange.set(true); checkbox.command("focus"); }, -1),
    action("Restore checkbox", () => { removeOnChange.set(false); visible.set(true); }, -1),
    Probe({ get snapshot() { return { scene: "checkbox", visible: visible(), checked: checked(), mixed: mixed(), allow: allow(),
      disabled: disabled(), focuses: focuses(), blurs: blurs(), changes: changes(), requested: requested(), focused: focused(), errors: errors() }; },
      style: { width: 0, height: 0 },
    }),
  ];
});
