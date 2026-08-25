---
title: Dock Panels
description: Putting a script view into a docked layout — panels that survive a restart, an uninstalled application that keeps its place, and who draws the chrome.
order: 11
---

# Dock Panels

A plugin that can only fill a window is not much of a plugin. This is how a script view becomes a **panel** in a docked layout: draggable, dockable, zoomable, and still there after a restart.

::: warning The script side is not connected yet
Everything on this page works from Rust today. The two traits that would let a *script* declare its own panel hooks or draw the dock's chrome — `PanelScript` and `DockChrome` — are defined and tested, but no engine implements them yet, so a host that wants either writes it in Rust for now. The seam is drawn; the script side of it is [not there yet](#not-there-yet).
:::

## What base already provides

`gpui_base::dock` has the hard half of a plugin system already: a layout that is **pure data**, a `PanelRegistry` that rebuilds a panel from a name in a persisted file, and a per-panel `serde_json::Value` that rides along with it. What it lacks is a way for a panel to come from somewhere other than the host binary. That is what this module adds.

## A script view as a panel

```rust
use gpui_shell::dock::ScriptPanel;

let panel = ScriptPanel::new("quotes", script_view, cx)
    .with_closable(true)
    .with_zoomable(true);
```

`ScriptPanel` implements `gpui_base::dock::Panel` — behavior — and nothing above it. Its title, toolbar and menus are not its own: they are drawn by the skin below, from the script's elements. That is what "the script owns presentation" means once a panel is involved.

The name is interned and always prefixed, so a script panel can never collide with a host panel that happens to share a word.

## Surviving a restart

A layout is saved as data: which panels, where, and one JSON payload each. Getting a panel *back* needs a builder registered under its name:

```rust
let name = gpui_shell::dock::register_panel("todolist", "quotes", script, cx);
```

Call it before `DockArea::load` runs. The builder invokes `PanelScript::build` to make a fresh script view, then hands the persisted payload to `PanelScript::deserialize`. Three hooks cross the boundary and no more:

| Hook | When | Note |
| --- | --- | --- |
| `build` | The registry is rebuilding the panel | `None` means the script could not be instantiated; the payload is carried forward untouched rather than dropped |
| `serialize` | The layout is being saved | Takes `&App`, not `&mut Window` — a read, with no call scope, so the script's `serialize()` must return a plain value and call nothing back into the host |
| `deserialize` | Right after `build` | A real host call, so this one may open a scope and touch entities |

Everything else about a panel — where it sits, whether it is displayed, what it is called — is the layout's business and never reaches the script.

A `ScriptPanel` with no script hooks connected still works; it persists its position and nothing else.

## An uninstalled application keeps its place

This is the part worth knowing before shipping plugins.

If an application is *not* loaded, nothing is registered under its name and `DockArea::load` finds no builder for its panels. It does not drop them. Base substitutes a draw-nothing placeholder that answers `Panel::dump` with the state it was handed, so the next save writes the panel — name, payload and position — back out unchanged.

Uninstall an application, use the window for a week, reinstall it: its panels come back where they were, with the state they had. This module keeps that promise one step further in — a panel that *is* registered but whose `build` fails is carried forward the same way, rather than losing its state to a broken script.

## Who draws the chrome

Base draws **no chrome at all**. An area with no renderer docks, drags, resizes and persists while painting nothing but the panels — no tab bar, no dock frame, no drag handle. Every piece of that has to come back through the three renderer traits, which `ScriptDockSkin` forwards to one `DockChrome`:

```rust
use gpui_shell::dock::ScriptDockSkin;

dock_area.with_renderer(ScriptDockSkin::new(chrome));
```

| `DockChrome` method | Draws |
| --- | --- |
| `tab_bar` | The tab bar above a group's displayed panel |
| `empty_group` | What a group with no displayed panel shows |
| `drop_indicator` | Where a dragged panel would land |
| `dock` | A dock's frame around its content — title strip, collapse, resize handle |
| `tile_drag_bar` | The strip a tile is dragged by, at base's fixed `DRAG_BAR_HEIGHT` |
| `tile_resize_handles` | A tile's resize affordances, at base's `HANDLE_SIZE` |

Every method receives the **resolved** context — never a drag event, a mouse position or a hit test, because base attaches all of that to the elements it gets back. The job is to turn state into elements and to call the context's own callbacks (`select_tab`, `close`, `toggle_zoom`, `resize_to`) rather than reimplement them.

`ScriptDockSkin::default()` is a skin that draws nothing, which is also base's own behavior: a working dock with bare panels.

For the eventual script side, `tab_group_data`, `dock_data` and `tile_data` convert each context's state half into plain JSON — the form an engine would hand to script code.

## Not there yet

- **The script side of both traits.** No engine implements `PanelScript` or `DockChrome`, so a script cannot yet declare `serialize()` / `deserialize(data)` for its panel, nor draw a tab bar. The Rust traits and the JSON converters are in place for when one does.
- **A panel a script can open by itself.** Panels are created by the host; there is no `cx.open_panel(...)` on the script side.
- **Layout mutation from script.** Moving, splitting or closing panels is the host's, through base's own API.
