// A workspace: a dockable layout drawn entirely by script.
//
// `gpui-base` supplies the behaviour — splits, tab groups, docks, drag and
// drop, zoom, and a layout that is pure data — and draws no chrome at all. An
// area with no chrome still docks, drags, resizes and persists; it simply
// paints nothing but the panels. Everything you can see here is in `ui.js`.
//
//   cargo run -p gpui-shell -- examples/js_dock

import { View, div } from "gpui";
import { DockArea, dock_area, dock_content, v_flex } from "gpui-base";
/** @import { AsyncContext, Context } from "gpui" */
import { BAR, dockBar, dockHandle, dockTab, dropHint, emptyGroup, label, muted } from "./ui.js";

/** Where the layout is kept between runs. */
const LAYOUT = "workspace.layout";

/**
 * One panel's body.
 *
 * It is an ordinary view, and that is the point: a panel is a view that a dock
 * happens to be holding. The two extra methods are what carries its state
 * across a restart — `serialize()` is read when the layout is saved,
 * `deserialize(data)` is handed back what it wrote.
 */
class Document extends View {
  /** @param {{ caption?: string }} props */
  init(props) {
    this.caption = props?.caption ?? "Untitled";
    this.edits = 0;
  }

  // Runs without a host call: return a value and touch nothing else.
  serialize() {
    return { caption: this.caption, edits: this.edits };
  }

  /** @param {{ caption: string, edits: number }} data */
  deserialize(data) {
    this.caption = data.caption;
    this.edits = data.edits;
  }

  /** @param {Context} cx */
  render(cx) {
    return v_flex()
      .size_full()
      .p(16)
      .gap(8)
      .bg(cx.theme().colors.background)
      .child(label(this.caption, cx))
      .child(muted(this.edits + " edits", cx))
      .child(
        div()
          .id("edit-" + this.caption)
          .px(10)
          .py(6)
          .rounded(6)
          .text_size(12)
          .bg(cx.theme().colors.primary)
          .text_color(cx.theme().colors.primary_foreground)
          .on_click((_event, cx) => {
            this.edits += 1;
            cx.notify();
          })
          .child("Edit"),
      );
  }
}

/** A panel with no state of its own, to show one that needs no hooks. */
class Files extends View {
  /** @param {Context} cx */
  render(cx) {
    return v_flex()
      .size_full()
      .p(12)
      .gap(6)
      .bg(cx.theme().colors.background)
      .children(["main.js", "ui.js", "README.md"].map((name) => muted(name, cx)));
  }
}

export default class Workspace extends View {
  /** @param {unknown} _props @param {AsyncContext} cx */
  init(_props, cx) {
    // Registered before anything is loaded: this is what lets a saved layout
    // find the class again. Both panels are registered, including the one with
    // no serialize() — a panel with no payload still needs a way back.
    DockArea.register_panel("document", Document);
    DockArea.register_panel("files", Files);

    this.dock = DockArea.new("workspace", { version: 1 });
    this.saving = null;

    const saved = localStorage.getItem(LAYOUT);
    if (saved) {
      // Restores the tree, the dock sizes and every panel's own payload.
      this.dock.load(JSON.parse(saved));
    } else {
      this.dock.add_panel(cx.new(Files), { name: "files", placement: "left", size: 200 });
      this.dock.add_panel(cx.new(Document, { caption: "main.js" }), { name: "document" });
      this.dock.add_panel(cx.new(Document, { caption: "ui.js" }), { name: "document" });
    }

    // Fires on every edit, including each step of a drag — so the write is on a
    // timer rather than on the event.
    this.dock.on("layout_changed", (cx) => {
      cx.notify();
      if (this.saving) return;
      this.saving = cx.timer.after(500, () => {
        this.saving = null;
        localStorage.setItem(LAYOUT, JSON.stringify(this.dock.dump()));
      });
    });
  }

  /** @param {Context} cx */
  render(cx) {
    return v_flex()
      .size_full()
      .bg(cx.theme().colors.background)
      .child(this.toolbar(cx))
      .child(
        dock_area(this.dock)
          .flex_1()
          .tab_bar((group, cx) =>
            div()
              .id("tab-bar-" + group.node)
              .flex()
              .h(BAR)
              .w_full()
              .bg(cx.theme().colors.secondary)
              .border_b(1)
              .border_color(cx.theme().colors.border)
              // The bar itself accepts a drop, so a tab dragged onto it joins
              // this group at the end rather than splitting beside it.
              .drop_tab(group)
              .children(group.tabs.filter((each) => each.visible).map((each) => dockTab(group, each, cx))),
          )
          .empty_group((_group, cx) => emptyGroup(cx))
          .drop_indicator((drop, cx) => dropHint(drop, cx))
          // Whatever this returns replaces the dock's content, so the panels go
          // where `dock_content()` is.
          .dock((dock, cx) =>
            v_flex()
              .size_full()
              .relative()
              .bg(cx.theme().colors.background)
              .child(dockBar(dock, cx))
              .child(dock_content().flex_1().overflow_hidden())
              .child(dockHandle(dock, cx)),
          ),
      );
  }

  /** @param {Context} cx */
  toolbar(cx) {
    const open = this.dock.panels().filter((panel) => panel.placement === "center").length;
    return div()
      .flex()
      .h(BAR)
      .px(10)
      .gap(10)
      .items_center()
      .bg(cx.theme().colors.secondary)
      .border_b(1)
      .border_color(cx.theme().colors.border)
      .child(label("Workspace", cx))
      .child(muted(open + " open", cx))
      .child(
        div()
          .id("new-document")
          .px(8)
          .py(3)
          .rounded(4)
          .text_size(11)
          .text_color(cx.theme().colors.muted_foreground)
          .hover((it) => it.bg(cx.theme().colors.accent))
          .on_click((_event, cx) => {
            this.dock.add_panel(cx.new(Document, { caption: "note " + (open + 1) }), {
              name: "document",
            });
            cx.notify();
          })
          .child("New"),
      )
      .child(
        div()
          .id("reset-layout")
          .px(8)
          .py(3)
          .rounded(4)
          .text_size(11)
          .text_color(cx.theme().colors.muted_foreground)
          .hover((it) => it.bg(cx.theme().colors.accent))
          .on_click((_event, cx) => {
            localStorage.removeItem(LAYOUT);
            cx.notify();
          })
          .child("Forget layout"),
      );
  }
}
