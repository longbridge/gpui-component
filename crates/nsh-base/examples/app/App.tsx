import { mount, signal, Text, View } from "nsh";
import { Button, Checkbox, Input } from "nsh/ui/base";

function App() {
  const draft = signal("");
  const priority = signal(false);
  const inputFocused = signal(false);
  const buttonFocused = signal(false), checkboxFocused = signal(false);
  const created = signal(0);
  const last = signal<{ title: string; priority: boolean } | null>(null);
  let input: { command(name: string): void } | undefined;
  const ink = "#20242b", muted = "#656d78", accent = "#275fc2", line = "#d7dce3";

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
    <View style={{ flex: 1, padding: 32, backgroundColor: "#f7f8fa" }}>
      <View style={{ width: "100%", maxWidth: 440, gap: 24 }}>
        <View style={{ gap: 8 }}>
          <Text style={{ fontSize: 22, fontWeight: 600, color: ink }}>Create task</Text>
          <Text style={{ fontSize: 14, color: muted }}>Add a task to your workspace.</Text>
        </View>
        <View style={{ gap: 8 }}>
          <Text style={{ fontSize: 14, fontWeight: 600, color: ink }}>Task name</Text>
          <Input
            ref={(node) => { input = node; }}
            value={draft()}
            placeholder="What needs to be done?"
            accessibilityLabel="Task name"
            onChange={(value) => { if (value.length <= 80) draft.set(value); }}
            onFocus={() => inputFocused.set(true)}
            onBlur={() => inputFocused.set(false)}
            onSubmit={createTask}
            style={{
              width: "100%", height: 40, paddingHorizontal: 12, fontSize: 14, color: ink,
              backgroundColor: "#ffffff", borderWidth: 1, borderRadius: 6,
              borderColor: inputFocused() ? accent : line,
            }}
          />
          <Text style={{ fontSize: 12, color: muted }}>Up to 80 characters.</Text>
        </View>
        <Checkbox
          checked={priority()}
          accessibilityLabel="Priority"
          onChange={(checked) => priority.set(checked)}
          onFocus={() => checkboxFocused.set(true)}
          onBlur={() => checkboxFocused.set(false)}
          style={{ width: "100%", height: 32, borderWidth: 1, borderRadius: 4, borderColor: checkboxFocused() ? accent : "transparent" }}
        >
          <View style={{ flexDirection: "row", alignItems: "center", gap: 10, height: 32 }}>
            <View style={{
              width: 18, height: 18, borderRadius: 4, borderWidth: 1,
              borderColor: priority() ? accent : line,
              backgroundColor: priority() ? accent : "#ffffff",
              alignItems: "center", justifyContent: "center",
            }}>
              <Text style={{ fontSize: 12, fontWeight: 600, color: "#ffffff" }}>{priority() ? "✓" : ""}</Text>
            </View>
            <Text style={{ fontSize: 14, color: ink }}>Priority</Text>
          </View>
        </Checkbox>
        <Button
          accessibilityLabel="Create task"
          disabled={!draft().trim()}
          onPress={createTask}
          onFocus={() => buttonFocused.set(true)}
          onBlur={() => buttonFocused.set(false)}
          style={{
            width: 144, height: 36, borderRadius: 6, borderWidth: 2,
            borderColor: buttonFocused() ? "#142d55" : "transparent",
            backgroundColor: draft().trim() ? accent : "#e1e5eb",
          }}
        >
          <View style={{ width: "100%", height: 36, alignItems: "center", justifyContent: "center" }}>
            <Text style={{ fontSize: 14, fontWeight: 600, color: draft().trim() ? "#ffffff" : muted }}>Create task</Text>
          </View>
        </Button>
        <View style={{ borderTopWidth: 1, borderTopColor: line, paddingTop: 20, gap: 8 }}>
          <Text style={{ fontSize: 14, fontWeight: 600, color: ink }}>
            {created() ? `Created ${created()} ${created() === 1 ? "task" : "tasks"}` : "No tasks created yet."}
          </Text>
          <Text style={{ fontSize: 14, color: muted }}>
            {last() ? `${last()!.title} · ${last()!.priority ? "Priority" : "Standard"}` : "Your latest task will appear here."}
          </Text>
        </View>
      </View>
    </View>
  );
}

mount(App);
