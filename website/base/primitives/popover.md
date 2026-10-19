---
title: Popover
description: An anchored floating surface with controlled or internally managed open state.
order: 19
---

# Popover

An anchored floating surface with controlled or internally managed open state.

Like every `gpui-base` primitive, Popover supplies behavior and semantic structure without imposing a product visual language. Apply GPUI styles and compose the exported parts to match your design system.

## Example

The [single native Cargo entrypoint](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/native/src/bin/components.rs) selects this primitive from the [shared showcase implementation](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/mod.rs). The same showcase is compiled once for the WASM preview above.

```bash
cargo run -p gpui-base-examples -- popover
```

## Import

```rust
use gpui_kit::base::{Popover};
```

## Anatomy and API

The example composes `Popover`. GPUI's standard styling and event traits provide presentation; these base types provide the interaction structure.

The authoritative module is [`components/popover.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/popover.rs). Native and browser previews compile this same file.

## State and events

Use `.anchor(Anchor::TopCenter).offset(px(8.))` to open below the trigger,
centered, with an eight-pixel gap. The Base offset defaults to zero.
`Top*` anchors open below, `Bottom*` above, `LeftCenter` to the right,
and `RightCenter` to the left. The anchor names the popup's own point.
Window-edge clamping does not flip the popup or change its anchor.

The popup anchors to the trigger container, the element `Popover`'s own styles
apply to. In a block parent that container stretches to the parent's width; lay
it out in a flex row (or give it a width) to anchor the popup to the trigger.

`on_position` observes resolved popup and trigger bounds before content
prepaint for custom presentation. `.arrow(PopupArrow::new(px(6.)))` draws an
arrow from the content's edge, aimed at the point of the trigger the anchor
names, its base kept clear of `.corner_radius(..)`. The arrow is unstyled: its
background fills it and its border color strokes its slopes. It sits outside
the content, so include its depth in `offset`. `ArrowGeometry` gives the same
geometry (side, points, bounds) for drawing an arrow of your own. Styled
Component Popover draws this arrow with `.arrow(true)`.

Open state can be parent-controlled; activation, outside click, and Escape request lifecycle changes.
`.disabled(true)` ignores the trigger's presses and keys and keeps the popover
closed, even when `open(true)` asks for it; disable the trigger element itself as well.

Keep controlled state on the parent render type or in a GPUI entity. Update it in callbacks and call `cx.notify()`; do not recreate persistent entities during every render.

## Complete Rust example

The complete implementation used by the runnable showcase is embedded directly from Rust source:

<<< ../../../crates/base/examples/showcase/components/popover.rs{rust}

The command above supplies application initialization, window creation, and shared `BaseShowcase` state.

## Accessibility

Support Escape/outside dismissal and return focus; move focus only when its content requires it.

## Notes

Use stable element IDs where accepted. Verify focus, hover, active, selected, disabled, reduced-motion, and high-contrast appearances in the consuming design system.
