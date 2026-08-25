---
title: Dock Panels
description: The panel machinery that exists inside the crate, why it is not public yet, and what it will look like when it is.
order: 11
---

# Dock Panels

A plugin that can only fill a window is not much of a plugin. Turning a script view into a **panel** — draggable, dockable, zoomable, still there after a restart — is written and tested inside the crate.

::: warning Not public yet
`gpui_shell::dock` is crate-private. Nothing drives it: a script cannot yet contribute a panel, and no host loads a plugin, so publishing `ScriptPanel` or `register_panel` would be a promise about an API no caller has ever exercised. This page describes what is there and what it is waiting for — it is not something you can call today.
:::

## What base already provides

`gpui_base::dock` has the hard half of a plugin system already: a layout that is **pure data**, a `PanelRegistry` that rebuilds a panel from a name in a persisted file, and a per-panel `serde_json::Value` that rides along with it. What it lacks is a way for a panel to come from somewhere other than the host binary. That gap is what this module fills.

## The shape of it

`ScriptPanel` implements `gpui_base::dock::Panel` — behavior — and nothing above it. Its title, toolbar and menus are drawn by a skin, from the script's own elements, which is what "the script owns presentation" means once a panel is involved. Its name is interned and always prefixed, so a script panel cannot collide with a host panel that happens to share a word.

Getting a panel *back* after a restart needs a builder registered under its name. The builder makes a fresh script view, then hands it the persisted payload. Three hooks cross the language boundary and no more:

| Hook | When | Note |
| --- | --- | --- |
| `build` | The registry is rebuilding the panel | `None` means the script could not be instantiated; the payload is carried forward untouched rather than dropped |
| `serialize` | The layout is being saved | Takes `&App`, not `&mut Window` — a read, with no call scope, so a script's `serialize()` must return a plain value and call nothing back into the host |
| `deserialize` | Right after `build` | A real host call, so this one may open a scope and touch entities |

Everything else about a panel — where it sits, whether it is displayed, what it is called — is the layout's business and never reaches the script.

## An uninstalled application keeps its place

This is the part worth knowing before designing around it.

If an application is *not* loaded, nothing is registered under its name and `DockArea::load` finds no builder for its panels. It does not drop them. Base substitutes a draw-nothing placeholder that answers `Panel::dump` with the state it was handed, so the next save writes the panel — name, payload and position — back out unchanged.

Uninstall an application, use the window for a week, reinstall it: its panels come back where they were, with the state they had. This module keeps that promise one step further in — a panel that *is* registered but whose `build` fails is carried forward the same way, rather than losing its state to a broken script.

## Who would draw the chrome

Base draws **no chrome at all**. An area with no renderer docks, drags, resizes and persists while painting nothing but the panels — no tab bar, no dock frame, no drag handle. Every piece of that has to come back through three renderer traits, and `ScriptDockSkin` forwards all three to one `DockChrome`:

| `DockChrome` method | Draws |
| --- | --- |
| `tab_bar` | The tab bar above a group's displayed panel |
| `empty_group` | What a group with no displayed panel shows |
| `drop_indicator` | Where a dragged panel would land |
| `dock` | A dock's frame around its content — title strip, collapse, resize handle |
| `tile_drag_bar` | The strip a tile is dragged by, at base's fixed `DRAG_BAR_HEIGHT` |
| `tile_resize_handles` | A tile's resize affordances, at base's `HANDLE_SIZE` |

Every method receives the **resolved** context — never a drag event, a mouse position or a hit test, because base attaches all of that to the elements it gets back. The job is to turn state into elements and to call the context's own callbacks (`select_tab`, `close`, `toggle_zoom`, `resize_to`) rather than reimplement them. `tab_group_data`, `dock_data` and `tile_data` convert each context's state half into plain JSON — the form an engine would hand to script code.

A chrome that draws nothing is still a working dock, which is also base's own behavior.

## What it is waiting for

Two things, and the second is the one that matters:

- **An engine implementation of `PanelScript` and `DockChrome`.** Both are traits so that the script side can be written once per engine; neither has an implementation yet, so a script cannot declare `serialize()` / `deserialize(data)` for its panel or draw a tab bar.
- **A caller.** The module goes public when something drives it — a plugin model that mounts panels, or a host that asks for one. Publishing it before that would fix an API shape against no experience of using it.
