// Executed by the production NSH runtime; snapshots contain only NSH signals.
import { mount, signal } from "nsh";
import { nativeComponent } from "nsh/native";
import { insert } from "nsh:rt";
import * as Component from "nsh/ui/gpui";
const { Button, Toggle, ToggleGroup, RadioGroup, ButtonGroup, DropdownButton, Clipboard, Rating } = Component;
const Probe = nativeComponent("verify.probe", { props: ["snapshot"], events: ["command"] });
const box = { width: 380, height: 40 };
const display = (name, props, height = 28, children) => {
  const component = Component[name];
  const node = component({ ...props, style: { width: 380, height } });
  if (children) insert(node, () => children);
  return node;
};
mount(() => {
  const mode = signal("controls");
  const disabled = signal(false);
  const toggle = signal(false);
  const toggles = signal([]);
  const radio = signal("red");
  const buttons = signal([]);
  const rating = signal(6);
  const acceptRating = signal(false);
  const ratingRequests = signal([]);
  const changes = signal(0);
  const copied = signal("");
  const copyCount = signal(0);
  const exported = signal(null);
  const options = signal([
    { value: "bold", label: "Bold" },
    { value: "italic", label: "Italic" },
    { value: "mono", label: "Monospace", disabled: true },
  ]);
  const error = signal(null);
  const changed = (owner) => (value) => { owner.set(value); changes.update((n) => n + 1); };
  const onError = (value) => error.set(value);
  return [
    Button({ label: "Increment", style: box, onPress: () => changes.update((n) => n + 1) }),
    () => mode() === "controls" ? [
      Toggle({ label: "Emphasis", get checked() { return toggle(); }, get disabled() { return disabled(); }, outline: true, style: box, onChange: changed(toggle) }),
      ToggleGroup({ get items() { return options(); }, get value() { return toggles(); }, get disabled() { return disabled(); }, outline: true, segmented: true, style: box, onChange: changed(toggles), onError }),
      RadioGroup({ items: [{ value: "red", label: "Red" }, { value: "blue", label: "Blue" }, { value: "green", label: "Green", disabled: true }], vertical: false, get value() { return radio(); }, get disabled() { return disabled(); }, style: box, onChange: changed(radio), onError }),
      ButtonGroup({ items: [{ value: "small", label: "Small" }, { value: "large", label: "Large" }, { value: "auto", label: "Automatic", disabled: true }], get value() { return buttons(); }, get disabled() { return disabled(); }, multiple: true, style: box, onChange: changed(buttons), onError }),
      Rating({ get value() { return rating(); }, max: 7, get disabled() { return disabled(); }, style: box, onChange: (value) => { ratingRequests.update((requests) => [...requests, value]); changes.update((n) => n + 1); if (acceptRating()) rating.set(value); } }),
      Clipboard({ value: "NSH clipboard payload", accessibilityLabel: "Copy fixture", style: box, onCopied: (value) => { copied.set(value); copyCount.update((n) => n + 1); } }),
      DropdownButton({ label: "Export", items: [{ id: "pdf", label: "Export PDF" }, { separator: true }, { id: "html", label: "Export HTML" }], menuAccessibilityLabel: "Export options", style: box, onPress: () => changes.update((n) => n + 1), onSelect: (value) => exported.set(value) }),
    ] : [
      display("Icon", { path: "icons/check.svg", color: "success" }, 24),
      display("Avatar", { name: "Ada Lovelace", size: "sm" }, 28),
      display("AvatarGroup", { items: [{ name: "Ada" }, { name: "Grace" }, { name: "Alan" }], size: "sm", limit: 2, ellipsis: true }, 30),
      display("Badge", { count: 42 }, 32, display("Label", { label: "Inbox" }, 24)),
      display("Tag", { label: "Published", variant: "success", outline: true }, 28),
      display("Alert", { title: "Connection interrupted", message: "Reconnect to continue.", variant: "warning" }, 82),
      display("Marker", { label: "Today", variant: "separator", alignment: "center" }, 28),
      display("MarkerIcon", { icon: "icons/check.svg" }, 20),
      display("MarkerContent", { text: "Synchronized" }, 24),
      display("Spinner", { size: "sm" }, 28),
      display("Skeleton", { secondary: true }, 20),
      display("ShimmerText", { text: "Preparing document…", once: true }, 24),
      display("Progress", { value: 62, accessibilityLabel: "Download" }, 24),
      display("ProgressCircle", { value: 75, size: 40, accessibilityLabel: "Indexing" }, 44),
      display("Separator", { label: "Actions", dashed: true }, 28),
      display("Kbd", { keystroke: "cmd-shift-k", outline: true }, 26),
      display("Label", { label: "Search result", secondary: "document", highlights: "Search", prefixMatch: true }, 24),
      display("Link", { label: "GPUI Kit documentation", href: "https://gpui-kit.com" }, 24),
    ],
    Probe({ get snapshot() { return { fixture: "display", mode: mode(), disabled: disabled(), toggle: toggle(), toggles: toggles(), radio: radio(), buttons: buttons(), rating: rating(), ratingRequests: ratingRequests(), acceptRating: acceptRating(), changes: changes(), copied: copied(), copyCount: copyCount(), exported: exported(), order: options().map((o) => o.value), error: error() }; }, style: { width: 0, height: 0 }, onCommand: (command) => {
      if (command === "accept-rating") acceptRating.set(true);
      if (command === "clear-count") changes.set(0);
      if (command === "displays") mode.set("displays");
      if (command === "controls") mode.set("controls");
      if (command === "disable") disabled.set(true);
      if (command === "enable") disabled.set(false);
      if (command === "reorder") options.set([...options()].reverse());
      if (command === "reset") { toggle.set(false); toggles.set([]); radio.set("red"); buttons.set([]); rating.set(6); }
    } }),
  ];
});
