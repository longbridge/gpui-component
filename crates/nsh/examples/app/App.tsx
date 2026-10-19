import { mount, signal, Scroll, Text } from "nsh";
import { Button, Checkbox, Dialog, Input, Notification, Select, Sheet, Slider, Switch, type SliderValue } from "../../js/index";

function App() {
  const count = signal(0);
  const enabled = signal(true);
  const name = signal("");
  const fruit = signal<string | null>("apple");
  const amount = signal<SliderValue>(25);
  const released = signal<SliderValue>(25);
  const dialogOpen = signal(false);
  const sheetOpen = signal(false);
  const notificationOpen = signal(false);
  return (
    <Scroll class="flex-1" contentContainerClass="p-6 gap-4">
      <Text class="text-xl font-semibold">GPUI Component</Text>
      <Text>{`Count: ${count()}`}</Text>
      <Text>Name</Text>
      <Input
        value={name()}
        accessibilityLabel="Name"
        onChange={(value) => name.set(value)}
        style={{ width: 280, height: 36 }}
      />
      <Text>{name() ? `Hello, ${name()}` : "Enter your name"}</Text>
      <Text>Fruit</Text>
      <Select
        items={[
          { value: "apple", label: "Apple" },
          { value: "orange", label: "Orange" },
          { value: "banana", label: "Banana", disabled: true },
        ]}
        value={fruit()}
        accessibilityLabel="Fruit"
        placeholder="Choose a fruit"
        searchable
        cleanable
        onChange={(value) => fruit.set(value)}
        style={{ width: 280, height: 36 }}
      />
      <Text>{`Selected: ${fruit() ?? "None"}`}</Text>
      <Text>{`Amount: ${amount()} · Released: ${released()}`}</Text>
      <Slider
        value={amount()}
        accessibilityLabel="Amount"
        onChange={(value) => amount.set(value)}
        onRelease={(value) => released.set(value)}
        style={{ width: 280, height: 36 }}
      />
      <Button
        label="Increment"
        disabled={!enabled()}
        onPress={() => count.update((value) => value + 1)}
        style={{ width: 160, height: 36 }}
      />
      <Checkbox
        label="Enable increment"
        checked={enabled()}
        onChange={(value) => enabled.set(value)}
        style={{ width: 220, height: 36 }}
      />
      <Switch
        label="Enable increment"
        checked={enabled()}
        onChange={(value) => enabled.set(value)}
        style={{ width: 220, height: 36 }}
      />
      <Button label="Selection details…" onPress={() => dialogOpen.set(true)} style={{ width: 220, height: 36 }} />
      <Button label="Details…" onPress={() => sheetOpen.set(true)} style={{ width: 220, height: 36 }} />
      <Button label="Show notification" onPress={() => notificationOpen.set(true)} style={{ width: 220, height: 36 }} />
      <Dialog
        open={dialogOpen()}
        title="Selection details"
        description={`Selected fruit: ${fruit() ?? "None"}`}
        confirmLabel="Done"
        onOpenChange={(open) => dialogOpen.set(open)}
      />
      <Sheet
        open={sheetOpen()}
        title="Details"
        description={`Amount: ${amount()}`}
        onOpenChange={(open) => sheetOpen.set(open)}
      />
      <Notification
        open={notificationOpen()}
        message="Example notification"
        onOpenChange={(open) => notificationOpen.set(open)}
      />
    </Scroll>
  );
}

mount(App);
