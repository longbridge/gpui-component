---
name: design-guides
description: Product and interaction design guidance for GPUI Component applications. Use when designing layouts, establishing visual hierarchy and spatial grammar, choosing colors/tokens, setting density/zoom, handling interaction states, choosing between Button and Link, crafting interface copy and confirmation dialogs, or running design review and accessibility checklists.
---

# GPUI Component Design Guides

The authoritative guide is published and maintained at:

**<https://longbridge.github.io/gpui-component/docs/design-guides.md>**

That URL serves the guide as raw Markdown. Fetch and read it before choosing
components or writing layout code. This file deliberately does not restate it —
the published guide is the single source of truth, and a summary kept here would
drift from it.

Human-readable version: <https://longbridge.github.io/gpui-component/docs/design-guides>

## What the guide covers

Design thesis and desktop defaults · designing around user tasks · reading
hierarchy, emphasis budget, semantic color roles, density and typography ·
spacing scale and alignment spines · base font as zoom control · window shells,
responsive layout, forms · component variants by meaning · the nine interaction
states and pointer rules · overlays, alerts, toasts, motion · dense tables,
trees, docks, command palettes · interface copywriting and confirmation dialogs
· accessibility and design-review checklists.

## Non-negotiables

These hold even if you cannot fetch the guide. Everything else: read the URL.

- **Desktop before web convention.** Preserve keyboard access, window chrome,
  menus, dense data views, resizable regions, persistent navigation.
- **`Button` vs `Link`.** `Button` for every in-app command — use the `ghost` or
  `outline` variant when it should read quietly. `Link` only for external URLs
  and email addresses.
- **Tokens before values.** No raw hex or `rgb(...)` in application UI; use
  `cx.theme()` semantic tokens. Use rem-based helpers so window zoom works.
  Spacing values quoted anywhere are the current default scale, not literals to
  repeat.
- **State must be visible.** Hover, focus, selection, disabled, loading,
  validation, and destructive states each need distinct, consistent treatment.
- **Overlays.** Escape dismisses the topmost surface and returns focus to its
  trigger.
- **Copy.** Name the object and the verb — `Delete “Roadmap”?` with a `Delete`
  button, not `Are you sure?` with `OK`.
