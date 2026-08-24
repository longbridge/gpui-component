// A release checklist.
//
// The task: decide whether this release can ship. Everything on screen serves
// that decision — what remains, what blocks it, and the one action that
// concludes it. Run with:
//
//   cargo run -p gpui-shell -- examples/js_checklist

import { View, div, h_flex, v_flex, text } from "gpui";
import { RELEASE, SECTIONS, FILTERS, matches } from "./checklist.js";
import {
  SPACE,
  button,
  checkbox,
  emptyState,
  label,
  muted,
  row,
  rule,
  sectionTitle,
  stat,
  surface,
  tag,
  title,
} from "./ui.js";

export default class Checklist extends View {
  init() {
    this.sections = SECTIONS.map((section) => ({
      ...section,
      items: section.items.map((item) => ({ ...item })),
    }));
    this.filter = "all";
    this.shipped = false;
  }

  get items() {
    return this.sections.flatMap((section) => section.items);
  }

  get done() {
    return this.items.filter((item) => item.done).length;
  }

  get blocking() {
    return this.items.filter((item) => item.blocking && !item.done).length;
  }

  setFilter(filter, cx) {
    this.filter = filter;
    cx.notify();
  }

  toggle(item, done, cx) {
    item.done = done;
    this.shipped = false;
    cx.notify();
  }

  clearDone(cx) {
    for (const item of this.items) {
      item.done = false;
    }
    this.shipped = false;
    cx.notify();
  }

  ship(cx) {
    this.shipped = true;
    cx.notify();
  }

  render() {
    const visible = this.sections
      .map((section) => ({
        ...section,
        items: section.items.filter((item) => matches(item, this.filter)),
      }))
      .filter((section) => section.items.length > 0);

    return v_flex()
      .size_full()
      .bg("background")
      .p(SPACE.xl)
      .gap(SPACE.lg)
      .child(this.header())
      .child(
        surface()
          .child(this.toolbar())
          .child(rule())
          .child(
            visible.length === 0
              ? emptyState(
                  this.filter === "done" ? "Nothing finished yet" : "Everything is done",
                  this.filter === "done"
                    ? "Tick an item to see it here."
                    : "Switch to All to review what shipped.",
                )
              : v_flex()
                  .flex_1()
                  .py(SPACE.sm)
                  .children(visible.map((section) => this.section(section))),
          ),
      )
      .child(this.footer());
  }

  header() {
    return h_flex()
      .items_start()
      .justify_between()
      .gap(SPACE.xl)
      .child(
        v_flex()
          .gap(SPACE.xs)
          .child(title("Release checklist"))
          .child(muted(`${RELEASE} · ${this.items.length} items`)),
      )
      .child(
        row()
          .gap(SPACE.xl)
          .child(stat(this.done, "done"))
          .child(stat(this.items.length - this.done, "open"))
          .child(stat(this.blocking, "blocking")),
      );
  }

  toolbar() {
    const filters = FILTERS.map((entry) =>
      button(
        `filter-${entry.id}`,
        entry.caption,
        (_event, cx) => this.setFilter(entry.id, cx),
        { variant: "ghost", selected: this.filter === entry.id },
      ),
    );

    return h_flex()
      .items_center()
      .justify_between()
      .px(SPACE.lg)
      .py(SPACE.md)
      .gap(SPACE.md)
      .child(h_flex().gap(SPACE.xs).children(filters))
      .child(
        button("clear", "Reset all", (_event, cx) => this.clearDone(cx), {
          variant: "ghost",
          disabled: this.done === 0,
        }),
      );
  }

  section(section) {
    const rows = section.items.map((item) =>
      checkbox(
        `item-${item.id}`,
        item.done,
        (done, cx) => this.toggle(item, done, cx),
        h_flex()
          .flex_1()
          .items_center()
          .justify_between()
          .gap(SPACE.md)
          .child(
            label(item.caption).when(item.done, (el) =>
              el.text_color("muted_foreground").line_through(),
            ),
          )
          .when(Boolean(item.blocking) && !item.done, (el) => el.child(tag("blocking", "blocking"))),
      ),
    );

    return v_flex()
      .py(SPACE.sm)
      .child(div().px(SPACE.lg).pb(SPACE.xs).child(sectionTitle(section.name.toUpperCase())))
      .children(rows);
  }

  footer() {
    const ready = this.blocking === 0;
    const reason = this.shipped
      ? `${RELEASE} marked ready.`
      : ready
        ? "No blocking items remain."
        : `${this.blocking} blocking ${this.blocking === 1 ? "item" : "items"} remaining.`;

    return h_flex()
      .items_center()
      .justify_between()
      .gap(SPACE.lg)
      .child(muted(reason))
      .child(
        button("ship", "Mark release ready", (_event, cx) => this.ship(cx), {
          variant: "primary",
          disabled: !ready || this.shipped,
        }),
      );
  }
}
