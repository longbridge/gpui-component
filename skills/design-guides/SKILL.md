---
name: design-guides
description: Product and interaction design guidance for GPUI Component applications. Use when designing layouts, establishing visual hierarchy and spatial grammar, choosing colors/tokens, setting density/zoom, handling interaction states, choosing between Button and Link, crafting interface copy and confirmation dialogs, or running design review and accessibility checklists.
---

# GPUI Component Design Guide

Use this guide before choosing components or writing layout code. It records the product judgment accumulated through years of GPUI Component desktop work: an interface should feel native, restrained, precise, and understandable without guesswork.

This is a normative guide. **Must** identifies a correctness or ecosystem constraint, **should** is the default that needs a concrete reason to override, and **may** is an optional technique. Component API documentation remains the authority for individual methods.

## Design Thesis

Build interfaces that feel native, quiet, and precise:
1. **Clarity before personality.** Make the primary task and next action clear before adding brand expression.
2. **Composition before invention.** Start with established components and compose them into product-specific workflows.
3. **Tokens before values.** Colors, radii, typography, and spacing should form a system. Avoid isolated literals.
4. **Desktop before web convention.** Preserve keyboard access, window chrome, menus, dense data views, resizable regions, and persistent navigation.
5. **State must be visible.** Hover, focus, selection, disabled, loading, validation, and destructive states need distinct and consistent treatment.

## References

| File | Topic & Scope |
| --- | --- |
| [`references/design-thesis.md`](references/design-thesis.md) | Design thesis, learning from Shadcn, desktop defaults vs web habits |
| [`references/task-and-workflow.md`](references/task-and-workflow.md) | Designing around user tasks and mental models, scoping visible actions |
| [`references/visual-hierarchy-and-tokens.md`](references/visual-hierarchy-and-tokens.md) | Reading hierarchy, emphasis budget, semantic color roles, density tiers, typography, elevation |
| [`references/spatial-grammar-and-alignment.md`](references/spatial-grammar-and-alignment.md) | Spacing scale table (`xxs` ~ `xxl`), alignment spines, exact pixel invariants, inside-before-outside rules |
| [`references/zoom-and-scale.md`](references/zoom-and-scale.md) | Base font as zoom control, relative `rem` scaling, avoiding direct `px(...)` |
| [`references/layout-patterns.md`](references/layout-patterns.md) | Stable window shells, responsive desktop windows, forms and settings |
| [`references/components-and-composition.md`](references/components-and-composition.md) | Component variants by meaning, standard semantic components, Base vs presentation |
| [`references/interaction-states.md`](references/interaction-states.md) | 9 interaction states table, pointer rules, desktop commands vs hover toolbars, `Button` vs `Link` semantics |
| [`references/overlays-and-motion.md`](references/overlays-and-motion.md) | Smallest surface rule, alerts vs toasts, footer design, purposeful motion |
| [`references/data-heavy-interfaces.md`](references/data-heavy-interfaces.md) | Designing dense tables, trees, lists, docks, and command palettes |
| [`references/interface-language.md`](references/interface-language.md) | Context economy, natural localization, button verbs, confirmation dialogs, ellipsis `…` |
| [`references/checklists-and-review.md`](references/checklists-and-review.md) | Accessibility checklist, AI interface rules, 8-point design review checklist |

## Quick Checklist: Key Invariants

- **Button vs Link**: `Button` for all in-app commands (use `ghost`/`outline` for quiet treatments); `Link` only for external URLs and emails.
- **Color & Sizing**: No raw hex/rgb colors; use `cx.theme()` tokens. Use rem-based layout helpers (`p_2()`, `gap_3()`, `text_sm()`).
- **Confirmation Dialogs**: State the object in title (`Delete “Roadmap”?`), use specific action verb (`Delete` vs generic `OK`/`Confirm`), omit "Are you sure..." filler.
- **Overlays**: Escape dismisses the topmost surface and restores focus to its trigger.
