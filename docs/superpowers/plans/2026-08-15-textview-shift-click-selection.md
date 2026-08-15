# TextView Shift+click Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend selectable TextView ranges from the last ordinary click when the user Shift+clicks or Shift+drags.

**Architecture:** Reuse the existing window selection `anchor` as the stable endpoint and `cursor` as the moving endpoint. Distinguish begin from extend only at mouse-down; all later movement continues through the existing cursor-only update path.

**Tech Stack:** Rust, GPUI mouse events, `gpui::test` visual tests.

## Global Constraints

- Ordinary click resets the anchor; Shift+click preserves a usable anchor.
- Repeated Shift+click and Shift+drag only move the cursor endpoint.
- Suppressing interactive controls must not retain or extend stale TextView selection.
- Existing double/triple click, modal scope, proxy endpoint, focus, scrolling, and copy behavior remain unchanged.

---

### Task 1: Shift+click and Shift+drag behavior

**Files:**
- Modify: `crates/ui/src/text/window_selection.rs`
- Test: `crates/ui/src/text/window_selection.rs`

**Interfaces:**
- Consumes: `WindowTextSelection.anchor`, `Root::text_selection_endpoint`, and the existing mouse-move update path.
- Produces: a begin/extend selection-start API used by `TextSelectionController`.

- [ ] **Step 1: Write failing behavior tests**

Add real pointer-event tests using a helper that emits `MouseDownEvent` and
`MouseUpEvent` with `Modifiers { shift: true, ..Default::default() }`. Assert
literal selected strings for click→Shift+click, repeated extension across the
anchor, Shift+drag, plain-click reset, no-anchor fallback, cross-view extension,
and a suppressing control.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
cargo test -p gpui-component text::window_selection::tests::shift --lib
```

Expected: the extension tests fail because capture clears the old endpoints and
mouse-down assigns both endpoints to the Shift+clicked position.

- [ ] **Step 3: Implement begin versus extend**

Introduce a private mode:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectionStart {
    Begin,
    Extend,
}
```

Update `Root::start_text_selection` to consume a resolvable event-local anchor
only for `Extend`, otherwise assign both endpoints. During capture, stage the
old anchor for Shift+click and then clear selection as before; suppressed or
stopped events therefore stay cleared, while unsuppressed Shift+clicks consume
the staged anchor in bubble handling.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
cargo test -p gpui-component text::window_selection::tests::shift --lib
```

Expected: all Shift-prefixed regression tests pass.

- [ ] **Step 5: Refactor comments and keep focused tests green**

Keep the start-mode branching local to `window_selection.rs`, document why
suppression is decided in bubble phase, and rerun the focused command.

- [ ] **Step 6: Verify the complete affected package**

Run:

```bash
cargo test -p gpui-component text::window_selection::tests --lib
cargo fmt --all -- --check
cargo check -p gpui-component
git diff --check
```

Expected: every command exits successfully with no failures or formatting
errors.
