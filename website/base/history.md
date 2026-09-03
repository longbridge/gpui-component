---
title: History
description: Browser-style navigation trails for application state.
order: 7
---

# History

`History<T>` is a browser-style linear trail of locations with back and forward navigation. It is independent of GPUI and leaves applying a returned value to the caller.

## Import

```rust
use gpui_base::History;
```

## Navigation trail

Push every location the user visits. The current entry is the last value in the trail. For example, after visiting `A -> B -> C`, `C` is current and going back returns the new current entry, `B`:

```rust
let mut history = History::new();
history.push("A");
history.push("B");
history.push("C");

assert_eq!(history.back(), Some("B"));
assert_eq!(history.current(), Some(&"B"));
```

`back()` never moves past the root entry; it returns `None` there. `forward()` restores the nearest entry that was left behind. Pushing a new entry after going back drops that forward branch, just as a browser does after opening a new page. `max_entries` bounds the root-to-current entries: lowering it removes the oldest active entries immediately, and moving forward at the limit removes the oldest active entry before restoring the next one.

`entries()` iterates from the root to the current entry. With the full `A -> B -> C` trail, it yields `A`, `B`, then `C`; `entries().rev()` yields `C`, `B`, then `A`. `forward_entries()` iterates from the nearest forward entry to the furthest. Use `retain` to remove invalid locations, `replace_current` to update the current location in place, and `remove_current` to remove it without discarding the forward branch.

| Method | Does |
| --- | --- |
| `new()` | Creates an empty trail. `max_entries` defaults to 1000. |
| `max_entries(n)` | Caps root-to-current entries and immediately removes the oldest excess entries. |
| `push(entry)` | Makes `entry` current and drops the forward branch. |
| `back()`, `forward()` | Move through the trail and return the resulting current entry. |
| `current()` | Returns the current entry. |
| `can_back()`, `can_forward()` | Report whether movement in that direction is available. |
| `entries()`, `forward_entries()` | Iterate the current trail and forward branch in navigation order. |
| `replace_current(entry)`, `remove_current()` | Update or remove the current entry. |
| `retain(keep)`, `clear()` | Remove rejected entries from both sides, or empty the trail. |
