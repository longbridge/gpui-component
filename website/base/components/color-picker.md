---
title: Color Picker
description: State and interaction foundations for selecting colors in a custom picker UI.
order: 8
---

# Color Picker

State and interaction foundations for selecting colors in a custom picker UI.

Like every `gpui-base` primitive, Color Picker supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/base_components.rs) selects this component from the [shared showcase implementation](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base --example base_components -- color-picker
```

## Import

```rust
use gpui_base::{ColorPickerState};
```

## Anatomy and API

The example composes `ColorPickerState`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/color_picker.rs`](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/components/color_picker.rs). Native and browser previews compile this same file.

## State and events

`ColorPickerState` owns the current color and interaction state. Retain its entity on the parent view.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/color_picker.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Provide a textual color value and keyboard controls; never communicate selection by color alone.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
