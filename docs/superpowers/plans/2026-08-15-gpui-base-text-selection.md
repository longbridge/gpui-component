# gpui-base Text Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement renderer-neutral window text selection in `gpui-base`, prove it with a plain adapter, and migrate TextView without changing its behavior.

**Architecture:** An implementation-private `WindowSelectionState` owns participant endpoints and event coordination. A public `TextSelectionLayer` owns lifecycle wiring, while the `TextSelection` namespace exposes window operations without revealing state. Each participant owns a `TextSelectionHandle`; plain runs use base geometry while TextView attaches advanced copy, focus, scrolling, and virtual-content adapters.

**Tech Stack:** Rust, GPUI entities/elements/hitboxes/TextLayout, `gpui::test` visual tests.

## Global Constraints

- `gpui-base` must not depend on `gpui-component`, TextView, Markdown, HTML, Dialog, or Sheet types.
- Every runnable checkpoint has one authoritative selection state.
- Existing TextView behavior and builders remain compatible.
- Old component-layer window methods may be deprecated only after forwarding to window selection.
- Endpoints use region-relative content coordinates; scope and document order are explicit.
- Production changes follow red-green-refactor and preserve unrelated `.github/workflows/release.yml` work.

---

### Task 1: Base window state, regions, and gesture contract

**Files:**
- Create: `crates/base/src/text_selection.rs`
- Modify: `crates/base/src/lib.rs`
- Test: `crates/base/src/text_selection.rs`

**Interfaces:**
- Produces: `TextSelection`, `TextSelectionRegion`, `SelectionScopeId`, `SelectionRegionFrame`, `SelectionSnapshot`, and `WindowTextSelection`.
- Consumes: GPUI window events, `GlobalState` suppression, and `AutoScroll` commands through region callbacks.

- [ ] **Step 1: Write failing base contract tests**

Create fake regions backed by `Entity<TextSelectionRegionState>`. Test begin,
update, end, stable Shift anchor, reversed extension, cross-region ordering,
scope exclusion, suppression, dead-region fallback, and clear/has/copy through
`WindowTextSelection`.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p gpui-base text_selection --lib
```

Expected: compilation fails because the new public types and methods do not yet
exist.

- [ ] **Step 3: Implement the minimum window interface**

Start with these renderer-neutral types:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SelectionScopeId(u64);

#[derive(Clone)]
pub struct TextSelectionRegion(Entity<TextSelectionRegionState>);

pub struct SelectionRegionFrame {
    pub hitbox: Hitbox,
    pub bounds: Bounds<Pixels>,
    pub scroll_offset: Point<Pixels>,
    pub scope: SelectionScopeId,
    pub document_order: u64,
    pub text_bounds: Vec<Bounds<Pixels>>,
}

pub trait WindowTextSelection {
    fn selected_text(&mut self, cx: &mut App) -> String;
    fn has_text_selection(&mut self, cx: &mut App) -> bool;
    fn clear_text_selection(&mut self, cx: &mut App);
    fn end_text_selection(&mut self, cx: &mut App);
}
```

Port anchor/cursor, capture/bubble choreography, proxy endpoints, scope checks,
weak-region pruning, and logical copy ordering behind the window interface.

- [ ] **Step 4: Verify GREEN and refactor**

Run:

```bash
cargo test -p gpui-base text_selection --lib
cargo fmt --all -- --check
```

Expected: all base selection contracts pass.

---

### Task 2: Plain text-run adapter

**Files:**
- Modify: `crates/base/src/text_selection.rs`
- Test: `crates/base/src/text_selection.rs`
- Create: `crates/base/examples/selectable_text.rs`

**Interfaces:**
- Consumes: region selection snapshots from Task 1 and GPUI `TextLayout`.
- Produces: `SelectionRunFrame`, `SelectionRunState`, and a plain selectable-text example usable without TextView.

- [ ] **Step 1: Write failing run projection tests**

Use hand-derived UTF-8 strings and laid-out runs to test forward/reversed ranges,
multiple runs, multiple regions, empty gutters, Unicode boundaries, and plain
copy ordering.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p gpui-base text_selection::tests::plain --lib
```

Expected: compilation fails because run registration and projection are absent.

- [ ] **Step 3: Implement run projection**

Expose:

```rust
pub struct SelectionRunFrame {
    pub order: u64,
    pub text: SharedString,
    pub layout: TextLayout,
    pub bounds: Bounds<Pixels>,
}

pub struct SelectionRunState {
    pub byte_range: Option<Range<usize>>,
    pub active: bool,
}
```

Move the renderer-neutral geometric band calculation into base, keep ranges on
UTF-8 boundaries, cache selected substrings for plain copy, and demonstrate a
non-TextView selectable renderer in the example.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p gpui-base text_selection --lib
cargo check -p gpui-base --example selectable_text
```

Expected: plain adapter contracts and example compile successfully.

---

### Task 3: TextView advanced adapter and single state source

**Files:**
- Create: `crates/ui/src/text/selection_adapter.rs`
- Modify: `crates/ui/src/text/mod.rs`
- Modify: `crates/ui/src/text/text_view.rs`
- Modify: `crates/ui/src/text/inline.rs`
- Modify: `crates/ui/src/text/state.rs`
- Modify: `crates/ui/src/text/window_selection.rs`
- Modify: `crates/ui/src/root.rs`
- Modify: `crates/ui/src/global_state.rs`
- Test: `crates/ui/src/text/window_selection.rs`

**Interfaces:**
- Consumes: base window selection, region, snapshot, run registration, focus/scroll/copy adapter hooks.
- Produces: TextView integration backed only by window selection, including Markdown source and virtual block export.

- [ ] **Step 1: Add compatibility assertions before migration**

Keep the existing selection tests unchanged and add assertions that Root contains
no second selection after adapter integration. Add an integration fixture where
a base plain region and a TextView participate in one cross-region selection.

- [ ] **Step 2: Verify the compatibility baseline**

Run:

```bash
cargo test -p gpui-component text::window_selection::tests --lib
```

Expected: existing tests pass; the mixed-adapter test fails because TextView is
not registered with window selection.

- [ ] **Step 3: Implement `TextViewSelectionAdapter`**

Give each `TextViewState` a stable base region. Register its frame in
`TextView::paint`, register Inline runs during Inline paint, query base snapshots
for highlights, and attach callbacks for `selected_text_in`, focus, auto-scroll,
clear, and virtual block lookup.

- [ ] **Step 4: Remove the old authoritative state**

Delete Root's `WindowTextSelection`, selectable-view map, and inline-bounds map.
Move the old event lifecycle implementation to base, retain only TextView adapter
logic in `selection_adapter.rs`, and make every TextView selection query read the
window selection.

- [ ] **Step 5: Verify GREEN**

Run:

```bash
cargo test -p gpui-component text::window_selection::tests --lib
cargo test -p gpui-component text::text_view::tests --lib
```

Expected: the unchanged compatibility suite and mixed-adapter test pass.

---

### Task 4: Scope migration, deprecated forwarding, and completion

**Files:**
- Modify: `crates/ui/src/root.rs`
- Modify: `crates/ui/src/sheet.rs`
- Modify: `crates/ui/src/dialog.rs`
- Modify: `crates/ui/src/window_ext.rs`
- Modify: `crates/ui/src/text/window_selection.rs`
- Test: `crates/ui/src/text/window_selection.rs`

**Interfaces:**
- Consumes: `SelectionScopeId` and `WindowTextSelection` from base.
- Produces: opaque modal scope mapping and deprecated component-layer forwarding to the single window state.

- [ ] **Step 1: Write failing forwarding and scope tests**

Assert that base and deprecated component window methods observe the same
selection, clearing through either interface clears both, and active Dialog or
Sheet regions remain isolated.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test -p gpui-component text::window_selection::tests --lib
```

Expected: new base-forwarding assertions fail until the component methods use
the base extension.

- [ ] **Step 3: Implement scope and forwarding**

Map Root modal state to opaque scope IDs, move the scope marker to base, render
the base selection element automatically, and mark old selection methods deprecated
with migration notes. Use fully-qualified calls internally to avoid extension
trait ambiguity.

- [ ] **Step 4: Delete obsolete UI selection infrastructure**

Remove `ui::text::window_selection` once all remaining code is adapter-specific,
or reduce it to integration tests if Rust module locality requires that location.
There must be no second anchor/cursor or registration map in `gpui-component`.

- [ ] **Step 5: Run the completion gate**

Run:

```bash
cargo test -p gpui-base --lib
cargo test -p gpui-component --lib
cargo check --workspace
cargo fmt --all -- --check
git diff --check
```

Expected: every command exits successfully. Search additionally for old Root
selection fields and confirm no duplicate state remains.
