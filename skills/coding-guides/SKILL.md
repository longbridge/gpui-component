---
name: coding-guides
description: Architecture and coding conventions for maintainable GPUI Component applications. Use when organizing application crates, choosing between RenderOnce and Entity, managing state and async work, implementing theme tokens and rem zoom, designing public component APIs, naming types/methods, testing, or following rules for coding agents.
---

# GPUI Component Coding Guides

The authoritative guide is published and maintained at:

**<https://longbridge.github.io/gpui-component/docs/coding-guides.md>**

That URL serves the guide as raw Markdown. Fetch and read it before making
architecture, state-ownership, API, or lifecycle decisions. This file
deliberately does not restate it — the published guide is the single source of
truth, and a summary kept here would drift from it.

Human-readable version: <https://longbridge.github.io/gpui-component/docs/coding-guides>

## What the guide covers

Architecture layers and dependency direction · initialization order and `Root`
ownership · GPUI phases and contexts · choosing `RenderOnce` vs `Entity<T>` ·
state ownership, `cx.notify()`, `cx.emit()` · stable `ElementId` rules · Base vs
presentation boundary · theme tokens and rem zoom · events, Actions, focus ·
async lifecycle, scroll ownership, virtualization · public API design and naming
· testing strategy and performance.

## Non-negotiables

These hold even if you cannot fetch the guide. Everything else: read the URL.

- Read the nearest implementation, its tests, and the re-export seam before
  editing. Search the current source for signatures — never infer a method name
  by analogy from React, CSS, or an older GPUI example.
- Theme tokens (`cx.theme()`) over visual literals; rem-based scale helpers
  (`p_2()`, `gap_3()`, `text_sm()`) over `px(...)` in application layout.
- Repeated elements need stable, domain-derived `ElementId`s — not list indexes.
- Choose `RenderOnce` vs `Entity<T>` deliberately, and be able to name the
  behavior owner, the presentation owner, and the test that would fail if the
  behavior regressed.
- "It compiles" is not a UI quality bar. Generated code must be reviewed and
  tested by a person.
