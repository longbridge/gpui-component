---
title: Radio Group
description: Groups radio items and provides keyboard navigation for a single selection.
order: 22
---

# Radio Group

Groups radio items and provides keyboard navigation for a single selection.

Like every `gpui-base` primitive, Radio Group supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- radio-group
```

## Import

```rust
use gpui_kit::base::{Radio, RadioGroup};
```

## Anatomy and API

The example composes `Radio`, `RadioGroup`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/radio_group.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/radio_group.rs). Native and browser previews compile this same file.

## State and events

The group coordinates one selected value and keyboard movement; `Radio` renders each option.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Keyboard

`RadioGroup` owns the keyboard for the `Radio`s below it, wherever a skin places them, as platform radio groups do. The group is one Tab stop: Tab lands on the focused radio, otherwise the checked one, otherwise the first enabled radio. Every arrow key moves focus to the previous (Left, Up) or next (Right, Down) enabled radio, wrapping at either end, and requests its check through the radio's `on_change` with a keyboard click event. `RadioGroup::track_focus(&handle)` gives the group one handle of its own: focusing it moves focus on to the radio holding the Tab stop. An owner that moves between radios itself turns this off with `.keyboard_navigation(false)`; each enabled radio is then its own Tab stop.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/radio_group.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Label the group, expose one checked item, and support arrow keys among enabled choices.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
