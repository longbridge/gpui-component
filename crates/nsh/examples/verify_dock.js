import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import { DockArea as Dock, Resizable as Split, ResizeHandle as Handle } from "nsh/ui/gpui";
const Button = nativeComponent("gpui-component.Button", { props: ["label"], events: ["press"] });
const Input = nativeComponent("gpui-component.Input", {props: ["value", "accessibilityLabel"], events: ["change"]});
const Probe = nativeComponent("verify.probe", { props: ["snapshot"], events: ["command"] });
const RawSlot = nativeComponent("__nsh.Slot", { props: ["__nshSlot"] });
function slot(name, children) { const node = RawSlot({__nshSlot: name}); insert(node, () => children); return node; }
mount(() => {
  const count = signal(0), panelCount = signal(0), splitCount = signal(0), resized = signal([]), saved = signal(null), changes = signal(0), zoomed = signal(false), error = signal(null), handleDelta = signal(null), text = signal(""), active = signal("");
  let dock, split, editor;
  const panels = [{id: "alpha", title: "Alpha"}, {id: "beta", title: "Beta"}];
  const dockNode = Dock({panels, layout: {center: {axis: "horizontal", children: [{panels: ["alpha"], size: 300}, {panels: ["beta"]}]}}, tabBar: true, style: {width: 600, height: 280}, ref: node => dock = node,
    onActiveChange: value => { if (value.active) active.set(value.id); }, onLayoutChange: () => changes.update(n => n + 1), onSave: value => saved.set(value), onZoomChange: value => zoomed.set(value.zoomed), onError: value => error.set(value)});
  insert(dockNode, () => [slot("alpha", [Button({label: "Alpha action", style: {width: 120, height: 36}, onPress: () => panelCount.update(n => n + 1)}), Input({ref: node => editor = node, get value() { return text(); }, accessibilityLabel: "Dock editor", style: {width: 240, height: 36}, onChange: value => text.set(value)})]), slot("beta", Button({label: "Beta action", style: {width: 120, height: 36}, onPress: () => panelCount.update(n => n + 10)}))]);
  const splitNode = Split({panels: [{id: "left", size: 200, minSize: 100}, {id: "right", minSize: 100}], style: {width: 600, height: 180}, ref: node => split = node, onResize: sizes => resized.set(sizes), onError: value => error.set(value)});
  insert(splitNode, () => [slot("left", Button({label: "Left split action", style: {width: 120, height: 36}, onPress: () => splitCount.update(n => n + 1)})), slot("right", Button({label: "Right split action", style: {width: 120, height: 36}, onPress: () => splitCount.update(n => n + 10)}))]);
  return [Button({label: "Increment", style: {width: 120, height: 36}, onPress: () => count.update(n => n + 1)}), dockNode, splitNode, Handle({trailing: true, accessibilityLabel: "Standalone resize", style: {width: 600, height: 50}, onEnd: value => handleDelta.set(value.delta)}),
    Probe({get snapshot() { return {fixture: "dock", count: count(), panelCount: panelCount(), splitCount: splitCount(), resized: resized(), saved: saved(), changes: changes(), zoomed: zoomed(), handleDelta: handleDelta(), text: text(), active: active(), error: error()}; }, onCommand: command => {
      if (command === "select-alpha") dock.command("select", "alpha");
      if (command === "focus-editor") editor.command("focus", null);
      if (command === "save") dock.command("save", null);
      if (command === "restore") dock.command("load", saved());
      if (command === "close") dock.command("close", "beta");
      if (command === "zoom") dock.command("zoom", "alpha");
      if (command === "unzoom") dock.command("unzoom", null);
      if (command === "resize") split.command("resize", {index: 0, size: 250});
    }}),
  ];
});
