---
title: Input
description: A renderable text editor foundation with selection, cursor, masking, highlighting, and LSP hooks.
order: 14
---

# Input

A renderable text editor foundation with selection, cursor, masking, highlighting, and LSP hooks.

Like every `gpui-base` primitive, Input supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/base_components.rs) selects this component from the [shared showcase implementation](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base --example base_components -- input
```

## Import

```rust
use gpui_base::{Input, InputState};
```

## Anatomy and API

The example composes `Input`, `InputState`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/input.rs`](https://github.com/longbridge/gpui-component/blob/main/crates/base/examples/showcase/components/input.rs). Native and browser previews compile this same file.

## State and events

`InputState` is the source of truth for text, selection, cursor, and focus; subscribe to its events for value changes.

Use `InputState::multi_line(true)` for text areas that accept line breaks. The live example demonstrates both the compact single-line field and a vertically scrollable multi-line editor; both retain native caret, selection, keyboard, and IME behavior.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/input.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Use a visible label, preserve caret semantics, and expose validation separately from placeholder text.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
