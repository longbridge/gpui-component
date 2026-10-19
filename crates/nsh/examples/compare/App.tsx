import { mount, signal, Text, View } from "nsh";
import { Button, Checkbox, Input } from "nsh/ui/gpui";

function App() {
  const draft = signal("");
  const priority = signal(false);
  const created = signal(0);
  const last = signal<{ title: string; priority: boolean } | null>(null);
  let input: { command(name: string): void } | undefined;

  function createTask() {
    const title = draft().trim();
    if (!title) return;
    last.set({ title, priority: priority() });
    created.update((count) => count + 1);
    draft.set("");
    priority.set(false);
    input?.command("focus");
  }

  return (
    <View style={{ flex: 1, padding: 32 }}>
      <View style={{ width: "100%", maxWidth: 440, gap: 24 }}>
        <View style={{ gap: 8 }}>
          <Text style={{ fontSize: 22, fontWeight: 600 }}>Create task</Text>
          <Text style={{ fontSize: 14 }}>Add a task to your workspace.</Text>
        </View>
        <View style={{ gap: 8 }}>
          <Text style={{ fontSize: 14, fontWeight: 600 }}>Task name</Text>
          <Input
            ref={(node) => { input = node; }}
            value={draft()}
            placeholder="What needs to be done?"
            accessibilityLabel="Task name"
            onChange={(value) => { if (value.length <= 80) draft.set(value); }}
            onSubmit={createTask}
            style={{ width: "100%", height: 40 }}
          />
          <Text style={{ fontSize: 12 }}>Up to 80 characters.</Text>
        </View>
        <Checkbox
          label="Priority"
          checked={priority()}
          accessibilityLabel="Priority"
          onChange={(checked) => priority.set(checked)}
          style={{ width: "100%", height: 32 }}
        />
        <Button
          label="Create task"
          variant="primary"
          accessibilityLabel="Create task"
          disabled={!draft().trim()}
          onPress={createTask}
          style={{ width: 144, height: 36 }}
        />
        <View style={{ paddingTop: 20, gap: 8 }}>
          <Text style={{ fontSize: 14, fontWeight: 600 }}>
            {created() ? `Created ${created()} ${created() === 1 ? "task" : "tasks"}` : "No tasks created yet."}
          </Text>
          <Text style={{ fontSize: 14 }}>
            {last() ? `${last()!.title} · ${last()!.priority ? "Priority" : "Standard"}` : "Your latest task will appear here."}
          </Text>
        </View>
      </View>
    </View>
  );
}

mount(App);
