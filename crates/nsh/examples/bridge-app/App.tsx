import { mount, signal, View, Text } from "nsh";
import { nativeComponent } from "nsh/native";
import { Button, Checkbox, Dialog, Input, Slot } from "../../js/index";
import { componentWithChildren } from "../../js/bridge";
const Host = componentWithChildren("verify.host", {});
const Probe = nativeComponent<{ snapshot: object }, { advance: void }>("verify.probe", { props: ["snapshot"], events: ["advance"] });
function App() {
  const count = signal(0), name = signal(""), show = signal(true);
  const dialogOpen = signal(false);
  const increment = () => count.update((n) => n + 1);
  return <>
    <Host style={{ width: 440, height: 260 }} slots={{ header: <Button label="Slot increment" onPress={increment} /> }}>
      <View style={{ width: 240, height: 42 }} accessibilityLabel="NSH view increment" accessibilityRole="button" onPress={increment}>
        <Text>Plain NSH subtree</Text>
      </View>
      {() => show() ? <>
        <Button label="Child increment" onPress={increment} />
        <Input value={name()} accessibilityLabel="Child input" onChange={(value) => name.set(value)} />
      </> : null}
    </Host>
    <Button label="Unmount children" style={{ width: 200, height: 36 }} onPress={() => show.set(false)} />
    <Button label="Open bridge dialog" style={{ width: 200, height: 36 }} onPress={() => dialogOpen.set(true)} />
    <Dialog open={dialogOpen()} title="Bridge dialog" onOpenChange={(open) => dialogOpen.set(open)} style={{ width: 0, height: 0 }}>
      <Slot>
        <Button label="Overlay geometry anchor" />
        <View style={{ height: 42 }} accessibilityRole="button" accessibilityLabel="Overlay NSH increment" onPress={increment}>
          <Text>{`Reactive overlay count: ${count()}`}</Text>
        </View>
        <Text>A regular NSH subtree receives the actual Component dialog width and keeps its original responsive layout and event handlers.</Text>
        <Button label="Overlay native increment" onPress={increment} />
      </Slot>
    </Dialog>
    <Input value="Locked" accessibilityLabel="Veto input" onChange={increment} style={{ width: 280, height: 36 }} />
    <Checkbox checked label="Veto checkbox" onChange={increment} style={{ width: 280, height: 36 }} />
    <Probe snapshot={{ count: count(), name: name(), show: show() }} onAdvance={increment} style={{ width: 0, height: 0 }} />
  </>;
}
mount(App);
