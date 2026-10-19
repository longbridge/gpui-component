// This fixture runs unchanged in NSH's JS runtime. Its probe reports signal
// values to the Rust verifier, independently of the controls' native state.
import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { Select, Slider } from "nsh/ui/gpui";

const Button = nativeComponent("gpui-component.Button", { props: ["label", "disabled"], events: ["press"] });
const Input = nativeComponent("gpui-component.Input", { props: ["value", "accessibilityLabel"], events: ["change"] });
const Checkbox = nativeComponent("gpui-component.Checkbox", { props: ["checked", "label"], events: ["change"] });
const Switch = nativeComponent("gpui-component.Switch", { props: ["checked", "label"], events: ["change"] });
const Probe = nativeComponent("verify.probe", { props: ["snapshot"] });

mount(() => {
  const count = signal(0);
  const name = signal("");
  const edits = signal(0);
  const checked = signal(true);
  const toggles = signal(0);
  const selection = signal("apple");
  const selections = signal(0);
  const searchable = signal(true);
  const items = signal([
    { value: "apple", label: "Apple" },
    { value: "orange", label: "Orange" },
    { value: "banana", label: "Banana", disabled: true },
  ]);
  const amount = signal(25);
  const releases = signal(0);
  const error = signal(null);
  const visible = signal(true);
  const box = { width: 280, height: 36 };
  return [
    Button({ label: "Increment", onPress: () => count.update((value) => value + 1), style: box }),
    () => visible() ? [
      Input({
        get value() { return name(); }, accessibilityLabel: "Name", style: box,
        onChange: (value) => { edits.update((value) => value + 1); name.set(value); },
      }),
      Checkbox({
        get checked() { return checked(); }, label: "Shared flag", style: box,
        onChange: (value) => { toggles.update((value) => value + 1); checked.set(value); },
      }),
      Switch({ get checked() { return checked(); }, label: "Mirrored flag", style: box,
        onChange: (value) => { toggles.update((value) => value + 1); checked.set(value); },
      }),
      Select({
        get value() { return selection(); }, get items() { return items(); },
        accessibilityLabel: "Fruit", get searchable() { return searchable(); }, style: box,
        onChange: (value) => { selections.update((value) => value + 1); selection.set(value); },
        onError: (value) => error.set(value),
      }),
      Slider({
        get value() { return amount(); }, accessibilityLabel: "Amount", style: box,
        min: 0, max: 100, step: 1, scale: "linear",
        onChange: (value) => amount.set(value),
        onRelease: () => releases.update((value) => value + 1),
        onError: (value) => error.set(value),
      }),
    ] : [],
    Button({ label: "Reorder options", style: box, onPress: () => items.set([...items()].reverse()) }),
    Button({ label: "Toggle search", style: box, onPress: () => searchable.set(!searchable()) }),
    Button({ label: "Reset controls", style: box, onPress: () => { name.set("Reset"); selection.set("apple"); amount.set(25); } }),
    Button({ label: "Unmount controls", style: box, onPress: () => visible.set(false) }),
    Probe({ get snapshot() { return {
      count: count(), name: name(), edits: edits(), checked: checked(), toggles: toggles(),
      selection: selection(), selections: selections(), amount: amount(), releases: releases(),
      itemOrder: items().map((item) => item.value),
      searchable: searchable(),
      error: error(), visible: visible(),
    }; }, style: { width: 0, height: 0 } }),
  ];
});
