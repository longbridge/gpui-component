---
title: History
description: A linear history with a cursor — undo and redo, back and forward, or a most-recent-first list — for any item a model wants to remember.
order: 7
---

# History

`History<I>` is a list of items with a cursor. `push` adds an item at the cursor, `undo` steps the cursor back and returns what it stepped over, `redo` steps it forward again. Pushing after an undo starts a new branch: the items ahead of the cursor are dropped, as a browser drops its forward pages when a new page is opened.

It holds no GPUI state and knows nothing about what the items mean. That is the point: the same type is an undo manager, a navigation trail, and a recent list, depending only on what you push into it.

## Import

```rust
use gpui_base::{History, HistoryItem};
```

An item is anything `Clone + PartialEq` that can carry a version number:

```rust
#[derive(Clone, PartialEq)]
struct Visit {
    symbol: String,
    version: usize,
}

impl HistoryItem for Visit {
    fn version(&self) -> usize { self.version }
    fn set_version(&mut self, version: usize) { self.version = version; }
}
```

The version is stamped by `push`. Items pushed within one grouping interval share a version and come back from `undo` together, so a drag that emits many small changes is undone as one step.

## Where it fits

**Undo and redo of changes.** Push each change; `undo` returns the changes to revert, `redo` the changes to reapply. The dock's tiles canvas records bounds changes this way, grouped by a short interval so one drag is one step. Use `start_grouping` and `end_grouping` when the boundary is known rather than timed.

**Back and forward through locations.** Push each place the user arrives at; `undo` is back and `redo` is forward, and `current()` is where they are. A place can stop being valid — its tab was closed — so `retain` prunes both sides of the cursor. A place recorded before it finished loading is corrected in place with `replace_current`, which keeps the version and the length, as a browser's `replaceState` does. [Nav Stack](./primitives/nav-stack.md) is built on this: its pages are the items, `pop` is `undo`, and `forward` is `redo`.

**A most-recent-first list.** With `unique()` a pushed item that is already present moves to the front instead of appearing twice, and `max_undos` caps the length. The stocks a user opened, the files they touched, the commands they ran: push on every use and read `undos()` back to front.

## API

| Method | Does |
| --- | --- |
| `new()` | An empty history. `max_undos` defaults to 1000. |
| `max_undos(n)`, `unique()`, `group_interval(d)` | Builders: cap the length, keep one copy of each item, group pushes closer than `d` into one step. |
| `push(item)` | Records `item` at the cursor and drops everything ahead of it. |
| `undo()`, `redo()` | Step the cursor and return the items stepped over, newest first; `None` at either end. |
| `current()` | The item at the cursor. |
| `replace_current(item)` | Overwrites the item at the cursor, keeping its version. Pushes when empty. |
| `retain(keep)` | Drops the items `keep` rejects, on both sides of the cursor. |
| `undos()`, `redos()` | The items behind and ahead of the cursor, oldest first. |
| `start_grouping()`, `end_grouping()` | Hold the version so the pushes in between undo as one step. |
| `set_ignoring(bool)`, `is_ignoring()` | A flag for the caller to skip recording while replaying its own history. |
| `clear()` | Empties both sides. |

## Notes

`History` never mutates your model. It hands back items and the caller applies them, which is why the same type serves changes that must be reverted and places that must be revisited. Decide what an item is before deciding whether to group or dedupe: a change wants grouping, a place wants neither, a recent list wants `unique`.
