---
title: GPUI Base
description: Unstyled, accessible foundations for building GPUI design systems.
order: 1
---

# GPUI Base

`gpui-base` is the unstyled foundation of GPUI Component. It provides interaction behavior, controlled state, focus management, accessibility semantics, animation, virtual lists, and theme tokens while leaving layout and visual design to your application.

## Choose the right layer

| Use | When |
| --- | --- |
| `gpui-base` | You are building a design system and want to own every visual choice. |
| `gpui-component` | You want a complete set of styled, ready-to-use desktop components. |

The dependency points one way: `gpui-component` builds on `gpui-base`. Applications can use either layer directly.

## Principles

- **Behavior is built in.** Controls provide consistent pointer, keyboard, focus, and state behavior.
- **Presentation is yours.** Compose GPUI style methods and children without fighting default visuals.
- **Parts stay composable.** Primitives expose their meaningful subparts instead of hiding markup behind a monolith.
- **State stays explicit.** Controlled inputs report changes and your view owns the resulting state.

## Start building

Follow [Getting started](./getting-started.md), learn how to add [window-level text selection](./text-selection.md) to custom renderers, then explore the [primitive catalog](./primitives/index.md). Each primitive page includes a Rust snippet and a live WASM example backed by the same example crate that can run natively.
