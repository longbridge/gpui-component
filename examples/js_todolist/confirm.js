// A confirmation dialog, opened by the list when an action destroys work.

import { View, v_flex, h_flex, text } from "gpui";
import { SPACE, button, label, muted } from "./ui.js";

export default class ConfirmClear extends View {
  init(props) {
    this.count = props?.count ?? 0;
    this.onConfirm = props?.onConfirm;
  }

  render() {
    return v_flex()
      .w(360)
      .bg("surface")
      .border(1)
      .border_color("border")
      .p(SPACE.xl)
      .gap(SPACE.md)
      .child(label(`Delete ${this.count} completed ${this.count === 1 ? "item" : "items"}?`))
      .child(muted("This cannot be undone."))
      .child(
        h_flex()
          .justify_end()
          .gap(SPACE.sm)
          .pt(SPACE.sm)
          .child(button("cancel", "Cancel", (_event, cx) => cx.close_dialog(), { variant: "ghost" }))
          .child(
            button(
              "confirm",
              "Delete",
              (_event, cx) => {
                if (this.onConfirm) this.onConfirm();
                cx.close_dialog();
              },
              { variant: "primary" },
            ),
          ),
      );
  }
}
