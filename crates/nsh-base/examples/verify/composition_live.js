import { mount, signal, View, Text } from "nsh";
import { Button } from "nsh/ui/base";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
const Host = nativeComponent("verify.host", { props: [] });
const Slot = nativeComponent("__nsh.Slot", { props: ["__nshSlot"] });
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });
mount(() => {
  const shown = signal(true), header = signal(0), body = signal(0), outer = signal(0), child = signal(0);
  const liveButton = (prefix, counter) => Button({
    get accessibilityLabel() { return `${prefix} ${counter()}`; },
    style: { width: 240, height: 44, backgroundColor: "#bfdbfe" },
    children: Text({ get children() { return `${prefix} ${counter()}`; } }),
    onPress: () => counter.update(n => n + 1),
  });
  const host = Host({ style: { width: 320, height: 160 } });
  const headerSlot = Slot({ __nshSlot: "header" });
  insert(headerSlot, () => shown() ? liveButton("Header", header) : []);
  insert(host, headerSlot);
  insert(host, () => shown() ? View({ style: { width: 280, height: 80 }, children: liveButton("Body", body) }) : []);
  const control = (label, onPress) => Button({ accessibilityLabel: label,
    style: { width: 280, height: 36, backgroundColor: "#e2e8f0" },
    children: Text({ children: label }), onPress,
  });
  return [
    Button({ accessibilityLabel: "Outer press region", style: { width: 320, height: 100, backgroundColor: "#e2e8f0" },
      onPress: () => outer.update(n => n + 1),
      children: View({ style: { width: 280, height: 70, backgroundColor: "#bbf7d0" },
        accessibilityRole: "button", accessibilityLabel: "NSH child action",
        onPress: () => child.update(n => n + 1), children: Text({ children: "NSH child action" }),
      }),
    }),
    host,
    control("Unmount live subtrees", () => shown.set(false)),
    control("Remount live subtrees", () => shown.set(true)),
    Probe({ get snapshot() { return { scene: "composition-live", shown: shown(), header: header(), body: body(), outer: outer(), child: child() }; }, style: { width: 0, height: 0 } }),
  ];
});
