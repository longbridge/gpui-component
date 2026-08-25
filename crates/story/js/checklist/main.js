// The JavaScript half of the "Shell" story.
//
//   cargo run   →   Gallery   →   Shell
//
// This file is read from disk when the story opens and again every time the
// "Reload script" button next to this panel is pressed. Editing it changes what
// the right-hand side of that window draws, with no `cargo build` in between —
// which is the entire argument for a script layer.
//
// It owns no state. The checklist lives in a Rust `Entity<Checklist>` that the
// panel on the left renders directly; this file reaches it through the two
// native modules `shell_story.rs` registered before the runtime started:
//
//   native("checklist")   steps() · reading() · toggle(id) · set_all(done)
//   native("theme")       palette()
//
// Both are plain data in, plain data out. There is no host object to hold on
// to here: a native call takes numbers, strings and records and gives them
// back, so nothing this file writes can outlive the call that produced it.

import { h_flex, v_flex, View, native } from "gpui";
import {
  SPACE,
  action,
  bar,
  label,
  marker,
  muted,
  palette,
  refreshPalette,
  row,
  rule,
  stepButton,
  surface,
  title,
} from "./ui.js";

export default class ChecklistBoard extends View {
  render() {
    // Read once per frame; see `refreshPalette` for why it is not cached.
    refreshPalette();

    const checklist = native("checklist");
    const steps = checklist.steps();
    const done = steps.filter((step) => step.done).length;

    // The live reading. Painting it is what makes the story's data feed visible
    // here: when the feed moves it, the Rust side notifies, this render runs
    // again, and the number changes. When the feed is only asking for repaints,
    // this render does not run at all and the number holds still — which is the
    // whole point of the counters under the panel.
    const reading = checklist.reading();

    return surface()
      .child(this.heading(steps.length, done, reading))
      .child(bar(steps.length === 0 ? 0 : done / steps.length))
      .child(rule())
      .child(this.list(checklist, steps))
      .child(rule())
      .child(this.actions(checklist, steps.length, done));
  }

  heading(total, done, reading) {
    return h_flex()
      .w_full()
      .items_start()
      .justify_between()
      .gap(SPACE.sm)
      .child(
        v_flex()
          .gap(SPACE.xxs)
          .child(title("Release checklist"))
          .child(muted("Drawn by main.js · state read over native(\"checklist\")")),
      )
      .child(
        v_flex()
          .items_end()
          .gap(SPACE.xxs)
          .child(label(`${done} / ${total}`))
          .child(muted(`reading ${reading}`)),
      );
  }

  list(checklist, steps) {
    if (steps.length === 0) {
      return muted("The Rust panel is holding no steps.");
    }

    return v_flex()
      .w_full()
      .gap(SPACE.xxs)
      .children(steps.map((step) => this.row(checklist, step)));
  }

  row(checklist, step) {
    return stepButton(`step-${step.id}`, `Toggle ${step.title}`, () => {
      // Deliberately no `cx.notify()`. The native call asks Rust to change the
      // checklist, and the Rust side notifies its observers — which re-renders
      // this view *and* the panel next to it. One change, one notification,
      // both halves in step.
      checklist.toggle(step.id);
    })
      .child(
        row()
          .child(marker(step.done))
          .child(
            label(step.title).when(step.done, (el) =>
              el.text_color(palette().muted_foreground).line_through(),
            ),
          ),
      )
      .child(muted(step.owner));
  }

  actions(checklist, total, done) {
    return h_flex()
      .w_full()
      .items_center()
      .justify_between()
      .gap(SPACE.sm)
      .child(muted(done === total ? "Ready to ship" : `${total - done} left`))
      .child(
        h_flex()
          .gap(SPACE.xs)
          .child(
            action("mark-all", "Mark all done", () => checklist.set_all(true), {
              primary: true,
              disabled: done === total,
            }),
          )
          .child(
            action("clear-all", "Clear all", () => checklist.set_all(false), {
              disabled: done === 0,
            }),
          ),
      );
  }
}
