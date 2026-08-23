---
name: coding-guides
description: Architecture and coding conventions for maintainable GPUI Component applications. Use when organizing application crates, choosing between RenderOnce and Entity, managing state and async work, implementing theme tokens and rem zoom, designing public component APIs, naming types/methods, testing, or following rules for coding agents.
---

# GPUI Component Coding Guide

This guide describes the application architecture and code patterns that have proved durable in GPUI Component. It is written for both engineers and coding agents.

This is a normative guide. **Must** marks lifecycle, correctness, or ecosystem constraints; **should** is the default architecture and requires a concrete reason to depart from it. Current source and API docs remain authoritative for exact signatures.

## Architecture & Core Tenet

Dependencies point downward. Higher layers own domain meaning and orchestration; lower layers own reusable presentation or behavior:
- **app shell**: compose windows and feature crates while keeping feature logic out;
- **feature crate**: keep one capability's model, services, views, commands, dialogs, and workflow behind one public boundary;
- **app component**: a repeated domain-aware pattern;
- **gpui-component**: themed, general-purpose UI;
- **gpui-base**: reusable behavior and geometry without product presentation.

## References

| File | Topic & Scope |
| --- | --- |
| [`references/architecture.md`](references/architecture.md) | Architecture layers, organizing applications by capability crates, dependency direction |
| [`references/bootstrap-and-root.md`](references/bootstrap-and-root.md) | Initialization order, `Root` view ownership, window-level facilities |
| [`references/units-and-lifecycle.md`](references/units-and-lifecycle.md) | GPUI phases & contexts, choosing between `RenderOnce` and `Entity<T>`, elements vs views vs behavior systems |
| [`references/state-ownership.md`](references/state-ownership.md) | State ownership, controlled values, `cx.notify()`, `cx.emit()`, avoiding feedback loops |
| [`references/stable-identity.md`](references/stable-identity.md) | `ElementId` rules, namespacing child IDs, transition channels, keys |
| [`references/rendering-and-composition.md`](references/rendering-and-composition.md) | Declarative rendering, fluent traits, Base vs presentation boundary |
| [`references/theme-and-zoom.md`](references/theme-and-zoom.md) | Theme tokens, avoiding hardcoded `px`/colors, base font as rem zoom control |
| [`references/events-and-focus.md`](references/events-and-focus.md) | Events, unified Actions, `FocusHandle`, focus trapping, `Button` vs `Link` semantics |
| [`references/async-and-data.md`](references/async-and-data.md) | Async lifecycle, layout & measurement, one scroll owner, list & table virtualization |
| [`references/api-and-naming.md`](references/api-and-naming.md) | Public API design, naming patterns table, precise domain words, callbacks, doc style |
| [`references/testing-and-performance.md`](references/testing-and-performance.md) | Testing strategy across layers, performance rules, common failure modes |
| [`references/agent-rules.md`](references/agent-rules.md) | Mandatory rules for coding agents & implementation checklist |

## Rules for Coding Agents

Before editing, an agent must read the nearest implementation, its tests, the re-export seam, and the relevant component documentation. It must search the current source for signatures instead of translating a React, CSS, or old GPUI example by analogy.

For each change, the agent should be able to name:
1. the behavior owner and presentation owner;
2. the retained identity and state lifecycle;
3. the pointer, keyboard, focus, and accessibility contract;
4. the layout and overflow owner;
5. the theme tokens and intentional exceptions;
6. the test that would fail if the behavior regressed.

Generated code must be reviewed and tested by a person. “Compiles” is not a UI quality bar, and a broad refactor that merely makes generated code look tidy is not a substitute for matching the repository's architecture.

## Implementation Checklist

Before opening a change for review, confirm that:
- [ ] State and side-effect ownership are explicit;
- [ ] `RenderOnce` versus `Entity<T>` is chosen deliberately;
- [ ] Repeated elements have stable domain-based IDs;
- [ ] Theme tokens and component sizes replace isolated visual literals;
- [ ] Keyboard actions, focus, disabled state, and overlays work together;
- [ ] Loading, empty, error, and cancellation paths are represented;
- [ ] Long data sets use an appropriate virtualized component;
- [ ] Public API additions preserve dependency direction and encapsulation;
- [ ] Tests prove behavior at the appropriate layer;
- [ ] Formatting, Clippy, targeted tests, and relevant examples pass.
