---
title: Virtual List
description: A high-performance vertical or horizontal list that renders only visible items.
order: 36
---

# Virtual List

A high-performance vertical or horizontal list that renders only visible items.

Like every `gpui-base` primitive, Virtual List supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/base_components.rs) selects this component from the [shared showcase implementation](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base --example base_components -- virtual-list
```

## Import

```rust
use gpui_base::{VirtualList, h_virtual_list, v_virtual_list};
```

## Anatomy and API

The example composes `VirtualList`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/virtual_list.rs`](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/components/virtual_list.rs). Native and browser previews compile this same file.

## State and events

The callback renders requested indexes only; item data and scroll state live outside it.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/virtual_list.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Preserve logical order, item counts, stable identity, and keyboard focus across virtualization.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
