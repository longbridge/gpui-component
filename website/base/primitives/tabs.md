---
title: Tabs
description: A tab list and accessible tab controls with controlled selection.
order: 30
---

# Tabs

A tab list and accessible tab controls with controlled selection.

Like every `gpui-base` primitive, Tabs supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- tabs
```

## Import

```rust
use gpui_kit::base::{Tab, Tabs};
```

## Anatomy and API

The example composes `Tab`, `Tabs`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/tabs.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tabs.rs). Native and browser previews compile this same file.

## State and events

The parent owns selected index/value; each tab reflects it and click handlers update the parent.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Keyboard

`Tabs` owns the keyboard for the `Tab`s below it, wherever a skin places them. The list is one Tab stop: Tab lands on the focused tab, otherwise the selected one, otherwise the first enabled tab. The arrow keys of its `axis` (Left and Right by default, Up and Down with `.axis(Axis::Vertical)`) move focus to the previous or next enabled tab, wrapping at either end, and Home and End move to the first and last. Moving focus selects the tab through its `on_click`, as platform tab views do; with `.activate_on_focus(false)` the arrows only move focus and Space or Enter selects. A click selects a tab without taking focus, so a document tab bar never takes focus from the document. `Tabs::track_focus(&handle)` gives the list one handle of its own: focusing it moves focus on to the tab holding the Tab stop.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/tabs.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Associate tabs with panels, expose selection, and support keyboard traversal.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
