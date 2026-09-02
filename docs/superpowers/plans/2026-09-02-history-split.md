# History and UndoHistory Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the overloaded public history type with a browser-style `History<T>` and a grouped `UndoHistory<T>`, then migrate every in-repository consumer.

**Architecture:** `History<T>` owns a root-to-current stack and a nearest-last forward stack; moving returns the destination and never backs past the root. `UndoHistory<T>` owns vectors of transactions and all grouping metadata, so stored changes no longer implement a version-bearing trait. Dock consumes `UndoHistory`, while NavStack consumes `History`.

**Tech Stack:** Rust 2024, `instant`, GPUI unit and component tests, VitePress Markdown documentation.

**Spec:** `docs/superpowers/specs/2026-09-02-history-split-design.md`

## Global Constraints

- Both `History<T>` and `UndoHistory<T>` are public from `gpui-base` and the legacy `gpui-component::history` path.
- Remove `HistoryItem` and the old overloaded API without deprecated aliases.
- `History::back` and `History::forward` return the destination entry.
- `History::back` never removes the root.
- `UndoHistory::undo` returns newest-first; `redo` returns oldest-first.
- Do not add a dependency for either data structure.
- Do not modify the sibling `longbridge-gpui` repository in this PR.

---

### Task 1: Browser-style History

**Files:**
- Replace: `crates/base/src/history.rs`

**Interfaces:**
- Produces: `History<T>` with `new`, `max_entries`, `push`, `current`, `replace_current`, `remove_current`, `can_back`, `can_forward`, `back`, `forward`, `entries`, `forward_entries`, `retain`, and `clear`.
- Consumes: only `std::vec::Vec`.

- [ ] **Step 1: Replace the old tests with failing navigation-contract tests**

Add plain Rust tests using integer entries. Cover this literal trail:

```rust
let mut history = History::new().max_entries(3);
history.push(1);
history.push(2);
history.push(3);

assert_eq!(history.current(), Some(&3));
assert_eq!(history.back(), Some(2));
assert_eq!(history.back(), Some(1));
assert_eq!(history.back(), None); // root remains current
assert_eq!(history.forward(), Some(2));
assert_eq!(history.entries().copied().collect::<Vec<_>>(), [1, 2]);
assert_eq!(history.entries().rev().copied().collect::<Vec<_>>(), [2, 1]);
assert_eq!(history.forward_entries().copied().collect::<Vec<_>>(), [3]);
```

Add separate tests for `back(); push(4)` truncating the forward entry, repeated
`1 -> 2 -> 1` entries remaining in order, a limit of two evicting the oldest
entry, a zero limit retaining nothing, `replace_current`, `remove_current`,
`retain` on both stacks, and `clear`.

- [ ] **Step 2: Run the History tests and verify RED**

Run:

```bash
cargo test -p gpui-base history::tests --lib
```

Expected: compilation fails because `max_entries`, `back`, `forward`, and the
iterator APIs do not exist and the old type requires `HistoryItem`.

- [ ] **Step 3: Implement the minimal navigation History**

Use this storage and movement model:

```rust
#[derive(Debug)]
pub struct History<T> {
    entries: Vec<T>,
    forward_entries: Vec<T>, // nearest entry is last
    max_entries: usize,
}

pub fn back(&mut self) -> Option<T>
where
    T: Clone,
{
    if self.entries.len() <= 1 {
        return None;
    }
    self.forward_entries.push(self.entries.pop().unwrap());
    self.current().cloned()
}

pub fn forward(&mut self) -> Option<T>
where
    T: Clone,
{
    let entry = self.forward_entries.pop()?;
    self.entries.push(entry);
    self.current().cloned()
}
```

Return `impl DoubleEndedIterator<Item = &T> + ExactSizeIterator` from both
iterator methods. Implement `Default` as `new`. On `push`, clear the forward
stack, skip storage when `max_entries == 0`, and evict index zero when at the
limit. `retain` applies to both stacks without reordering either.

- [ ] **Step 4: Run the History tests and verify GREEN**

Run:

```bash
cargo test -p gpui-base history::tests --lib
```

Expected: every new navigation history test passes.

- [ ] **Step 5: Commit the navigation history**

```bash
git add crates/base/src/history.rs
git commit -m "base: make History a navigation trail"
```

### Task 2: Grouped UndoHistory

**Files:**
- Create: `crates/base/src/undo_history.rs`
- Modify: `crates/base/src/lib.rs`

**Interfaces:**
- Consumes: `instant::{Duration, Instant}`.
- Produces: public `UndoHistory<T>` with `new`, `max_undos`, `group_interval`, `push`, `undo`, `redo`, `can_undo`, `can_redo`, `start_grouping`, `end_grouping`, `is_ignoring`, `set_ignoring`, and `clear`.

- [ ] **Step 1: Write failing transaction and ordering tests**

Create `undo_history.rs` with the test module first. Assert these hand-derived
orders:

```rust
let mut history = UndoHistory::new();
history.start_grouping();
history.push(1);
history.push(2);
history.push(3);
history.end_grouping();

assert_eq!(history.undo(), Some(vec![3, 2, 1]));
assert_eq!(history.redo(), Some(vec![1, 2, 3]));
```

Add independent tests proving ungrouped pushes form separate transactions,
timed grouping combines nearby pushes, a new push clears redo, ignore mode
drops pushes, `clear` clears both directions, `max_undos(2)` evicts the oldest
transaction, and `max_undos(0)` retains none.

- [ ] **Step 2: Run the UndoHistory tests and verify RED**

Run:

```bash
cargo test -p gpui-base undo_history::tests --lib
```

Expected: compilation fails because `UndoHistory` is not implemented.

- [ ] **Step 3: Implement transaction-owned grouping**

Use transaction storage rather than putting metadata in `T`:

```rust
#[derive(Debug)]
pub struct UndoHistory<T> {
    undos: Vec<Vec<T>>,
    redos: Vec<Vec<T>>,
    last_changed_at: Instant,
    max_undos: usize,
    group_interval: Option<Duration>,
    grouping: bool,
    ignoring: bool,
}
```

`push` appends to the last transaction only while explicit grouping is active
or the configured interval has not elapsed. Otherwise it creates a transaction.
It clears redo only for a recorded push. `undo` moves the stored transaction to
redo and returns a reversed clone; `redo` moves it back and returns an
oldest-first clone. Implement `Default` as `new`.

- [ ] **Step 4: Run UndoHistory tests and verify GREEN**

Run:

```bash
cargo test -p gpui-base undo_history::tests --lib
```

Expected: every transaction, ordering, grouping, ignore, and capacity test
passes.

- [ ] **Step 5: Commit UndoHistory**

```bash
git add crates/base/src/undo_history.rs crates/base/src/lib.rs
git commit -m "base: add grouped UndoHistory"
```

### Task 3: Migrate Dock, NavStack, Input, and compatibility exports

**Files:**
- Modify: `crates/base/src/nav_stack.rs`
- Modify: `crates/base/src/dock/tiles_state.rs`
- Modify: `crates/base/src/dock/tiles_geometry.rs`
- Modify: `crates/base/src/input/base/change.rs`
- Modify: `crates/base/src/lib.rs`
- Modify: `crates/ui/src/history.rs`
- Modify: `crates/ui/tests/base_compat.rs`

**Interfaces:**
- Consumes: `History<NavEntry>` from Task 1 and `UndoHistory<TileChange>` from Task 2.
- Produces: existing NavStack and Dock public behavior with no `HistoryItem` implementation anywhere in the workspace.

- [ ] **Step 1: Change compatibility and consumer tests to the new types**

In `base_compat.rs`, replace the `HistoryItem` fixture with a plain `u8` and
compile both legacy re-exports:

```rust
let _: gpui_component::history::History<u8> = gpui_base::History::new();
let _: gpui_component::history::UndoHistory<u8> = gpui_base::UndoHistory::new();
```

Keep the existing NavStack tests unchanged as consumer-level protection. Add a
Dock unit assertion only if its existing undo/redo tests do not cover grouped
tile changes and their application order.

- [ ] **Step 2: Run consumer tests and verify RED**

Run:

```bash
cargo test -p gpui-component --test base_compat legacy_history_path_reexports_the_base_type
cargo test -p gpui-base nav_stack --lib
```

Expected: compatibility compilation fails until `UndoHistory` is re-exported;
NavStack compilation fails because it still calls `undos`, `redos`, `undo`,
and `redo`.

- [ ] **Step 3: Migrate NavStack**

Remove `HistoryItem for NavEntry` and its `version`. Use `entries().len()`,
`entries()`, and `forward_entries()` for inspection. Implement pop by cloning
the outgoing top before `history.back()`; implement forward from the entry
returned by `history.forward()`. Remove the `undo_one` helper or replace it with
a helper that returns the outgoing view while using navigation semantics.

- [ ] **Step 4: Migrate Dock and remove stale item metadata**

Change the tile field to `UndoHistory<TileChange>`, preserving its 100 ms group
interval and its public `undo`/`redo` methods. Remove `TileChange.version` and
its trait implementation. Remove `Change.version`, its `HistoryItem`
implementation, and the now-unused import from Input.

- [ ] **Step 5: Finish exports and verify consumers GREEN**

Export both types from `gpui-base` and re-export both from
`gpui-component::history`. Run:

```bash
cargo test -p gpui-base nav_stack --lib
cargo test -p gpui-base dock --lib
cargo test -p gpui-component --test base_compat legacy_history_path_reexports_the_base_type
rg -n "HistoryItem|max_undos\(|\.undos\(|\.redos\(" crates --glob '*.rs'
```

Expected: all tests pass and the search returns only intentional
`UndoHistory::max_undos` references, with no `HistoryItem`, `undos()`, or
`redos()` call sites.

- [ ] **Step 6: Commit all consumers**

```bash
git add crates/base/src/nav_stack.rs crates/base/src/dock/tiles_state.rs crates/base/src/dock/tiles_geometry.rs crates/base/src/input/base/change.rs crates/base/src/lib.rs crates/ui/src/history.rs crates/ui/tests/base_compat.rs
git commit -m "base: migrate history consumers"
```

### Task 4: Rewrite History documentation and verify the branch

**Files:**
- Modify: `crates/base/README.md`
- Modify: `website/base/history.md`
- Modify: `website/zh-CN/base/history.md`

**Interfaces:**
- Consumes: the final APIs from Tasks 1 and 2.
- Produces: matching English and Chinese public documentation for both types.

- [ ] **Step 1: Rewrite the documentation around the split**

Document `History` first as a browser-style trail, with an `A -> B -> C`
example showing that `back()` returns `B`. Document `entries()` and
`entries().rev()` order. Document `UndoHistory` separately with a grouped drag
example and the newest-first undo/oldest-first redo contract. Remove all
references to `HistoryItem`, `unique`, and the claim that one type is also an
MRU list. Update the README catalog row to list both types and their distinct
purposes.

- [ ] **Step 2: Run fresh formatting and targeted verification**

Run:

```bash
cargo fmt --all -- --check
cargo test -p gpui-base history::tests --lib
cargo test -p gpui-base undo_history::tests --lib
cargo test -p gpui-base nav_stack --lib
cargo test -p gpui-base dock --lib
cargo test -p gpui-component --test base_compat
```

Expected: all commands exit zero with no failed tests.

- [ ] **Step 3: Run the broad compilation gate**

Run:

```bash
cargo check --workspace --all-targets
git diff --check origin/main...HEAD
```

Expected: the workspace check and whitespace check both exit zero.

- [ ] **Step 4: Commit documentation**

```bash
git add crates/base/README.md website/base/history.md website/zh-CN/base/history.md
git commit -m "docs: distinguish navigation and undo history"
```

- [ ] **Step 5: Prepare the independent pull request**

Push `history-split`, open a PR targeting `longbridge/gpui-component:main`, and
summarize the breaking API split, consumer migrations, and verification commands.
Confirm the PR diff begins at squash merge `0c746dff` from #2922 and contains
no commits from the former `nav-stack` branch.
