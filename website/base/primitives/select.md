---
title: Select
description: A button-like selection control backed by an anchored, keyboard-navigable popup.
order: 25
---

# Select

A button-like selection control backed by an anchored, keyboard-navigable popup.

Like every `gpui-base` primitive, Select supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- select
```

## Import

```rust
use gpui_kit::base::{Select};
```

## Anatomy and API

The example composes `Select`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/select.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/select.rs). Native and browser previews compile this same file.

## State and events

The delegate/state owns items and selection; activation opens the list and selection closes it.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/select.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Keyboard

Enter, Space and the arrow keys open a closed select and move focus to
`content_focus_handle`; Escape closes it and returns focus to the trigger. Tab
and Shift-Tab close an open select without choosing, as Escape does, then move
focus on from the trigger as usual.

Supply `.options(labels)` to let the root navigate the open list as well. Up and
Down move the highlighted option, stopping at either end; Page Up and Page Down
move it by a page, the rows the list shows (every option without a
`.scroll_handle(...)` to measure); and Home and End jump to the first and last
one; on a closed select they open it first, the arrows on
`.selected_index(...)`. Typing the start of a label highlights the next matching
option, opening a closed select; repeating one letter steps through the options
starting with it. Space typed within a second of the last letter goes on with
the search, so a label containing a space can be typed out; otherwise Space
activates. The application keeps `.highlighted_index(...)` in step from
`.on_highlight(...)`, marks that option with `aria_active_descendant()`, and
commits it in `.on_confirm(...)`, which runs with the content focused so it can
close the list and focus the trigger. `.scroll_handle(...)` scrolls the
highlighted option into view, and with it the pointer moving over an option
highlights it as well. Without options, those keys go on to the focused
content, which navigates on its own.

## Accessibility

Set `.accessibility_label(...)` on the controlled root and
`.accessibility_value(...)` to its committed selection, not a temporary search
cursor. The root exposes its expanded state and accessible activation. Activation
requests an open-state change and moves focus between the trigger and content.
Disabled controls do not expose activation. The styled `Select` supplies its
committed value automatically, falling back to its placeholder when unselected.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
