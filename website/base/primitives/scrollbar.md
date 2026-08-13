---
title: Scrollbar
description: An unstyled scrollbar connected to GPUI scroll or uniform-list handles.
order: 24
---

# Scrollbar

An unstyled scrollbar connected to GPUI scroll or uniform-list handles.

Like every `gpui-base` primitive, Scrollbar supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/base_components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base --example base_components -- scrollbar
```

## Import

```rust
use gpui_base::{Scrollbar};
```

## Anatomy and API

The example composes `Scrollbar`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/scrollbar.rs`](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/components/scrollbar.rs). Native and browser previews compile this same file.

## State and events

Bind it to the viewport's scroll handle; thumb and track interaction update that same handle.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/scrollbar.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Keep wheel, trackpad, and keyboard scrolling; give custom thumbs adequate size and contrast.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
