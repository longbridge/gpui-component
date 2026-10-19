import { signal, type YNode } from "nsh";
import { Button, DockArea, Resizable, type DockState } from "../../js/index";

/** NSH owns signals and commands; GPUI Component owns native dock mechanics. */
export function DockDemo() {
  const count = signal(0);
  const saved = signal<DockState | undefined>(undefined);
  let dock: YNode;
  return (
    <DockArea
      ref={(node) => { dock = node; }}
      style={{ width: 800, height: 560 }}
      panels={[
        { id: "files", title: "Files", closable: false },
        { id: "editor", title: "Editor", toolbar: [{ id: "save", label: "Save layout" }] },
        { id: "details", title: "Details" },
      ]}
      layout={{ center: { axis: "horizontal", children: [
        { panels: ["files"], size: 220 },
        { panels: ["editor", "details"], activeIndex: 0 },
      ] } }}
      onPanelAction={() => dock.command("save", null)}
      onSave={(state) => saved.set(state)}
      slots={{
        files: <Button label="Open editor" onPress={() => dock.command("select", "editor")} />,
        editor: <Resizable
          style={{ width: 560, height: 460 }}
          panels={[{ id: "content", minSize: 200 }, { id: "preview", size: 180, minSize: 120 }]}
          slots={{
            content: <Button label={`Count: ${count()}`} onPress={() => count.update((value) => value + 1)} />,
            preview: <Button label="Restore layout" disabled={!saved()} onPress={() => {
              const state = saved();
              if (state) dock.command("load", state);
            }} />,
          }}
        />,
        details: <Button label="Zoom details" onPress={() => dock.command("zoom", "details")} />,
      }}
    />
  );
}
