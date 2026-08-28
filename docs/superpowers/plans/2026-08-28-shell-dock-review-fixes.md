# Shell Dock Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the reviewed shell Dock API correctness, contract, resource-bound, validation, and frame-time regressions.

**Architecture:** Enforce retained-entity capacity at `EntityStore`, emit persistence events at the native mutation boundary, and reject invalid JavaScript before narrowing. Cache dock chrome as replayable spec arenas keyed by native container, callback, and payload so unchanged frames materialize without QuickJS.

**Tech Stack:** Rust, GPUI, rquickjs, serde_json, TypeScript declarations, Markdown, `gpui::test`.

**Spec:** `docs/superpowers/specs/2026-08-28-shell-dock-review-fixes-design.md`

## Global Constraints

- Preserve unrelated worktree changes, especially the existing `crates/shell/src/typings.rs` modification.
- Invalid JavaScript throws synchronously and is never clamped, defaulted, or queued.
- Locked means no rearrangement; resizing remains available.
- Cache spec data only, never consumed `AnyElement`s; cache successful null as an empty spec, but retry errors.
- Add no dependencies or unrelated public API.

---

### Task 1: Enforce the retained-entity limit at the store

**Files:**
- Modify/Test: `crates/shell/src/entities.rs`
- Modify: `crates/shell/src/engine/quickjs/entity_api.rs`
- Modify: `crates/shell/src/engine/quickjs/dock_api.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Test: `crates/shell/src/tests/dock.rs`

**Interfaces:**
- Produces: fallible `EntityStore::push` and retained constructors.
- Consumes: `MAX_LIVE_ENTITIES` and the existing retained-entity `RangeError` text.

- [ ] **Step 1: Write failing tests.** Add a store test proving insertion number `MAX_LIVE_ENTITIES + 1` returns an error, plus a public `DockArea.new` test asserting the thrown text contains `retained entity limit`.
- [ ] **Step 2: Verify RED.** Run `cargo test -p gpui-shell live_entity_limit` and confirm DockArea currently bypasses the limit.
- [ ] **Step 3: Implement the minimum fix.** Make `push` and every constructor return `Result`; reject before allocating the GPUI entity/id. Map the store error everywhere to:

```rust
Exception::throw_range(
    ctx,
    "the application reached gpui-shell's retained entity limit; release unused handles",
)
```

- [ ] **Step 4: Verify GREEN.** Run `cargo test -p gpui-shell live_entity_limit` and `cargo test -p gpui-shell engine::quickjs::entity_api::tests`.
- [ ] **Step 5: Commit.** Stage only the four files above and commit `fix(shell): enforce retained dock limits`.

### Task 2: Validate Dock JavaScript arguments

**Files:**
- Modify: `crates/shell/src/engine/quickjs/dock_api.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Test: `crates/shell/src/tests/dock.rs`

**Interfaces:**
- Produces: private finite-number and non-negative-safe-integer validators.
- Consumes: Dock new/register/add/remove/resize prelude and host calls.

- [ ] **Step 1: Write failing tests.** Cover `NaN`, infinity, negative/fractional versions and ids, negative/non-finite sizes, empty names, missing/wrong/non-finite bounds, and a function not extending `View`. Assert the method and argument are named; keep zero as a valid boundary.
- [ ] **Step 2: Verify RED.** Run `cargo test -p gpui-shell tests::dock::dock_arguments_are_validated_before_narrowing` and `cargo test -p gpui-shell tests::dock::register_panel_requires_a_view_subclass`.
- [ ] **Step 3: Implement validation.** Validate before every cast; use fallible bounds reads rather than `unwrap_or`; require non-empty `options.name`; require `Class.prototype instanceof View`, matching `cx.new`.
- [ ] **Step 4: Verify GREEN.** Run `cargo test -p gpui-shell tests::dock`.
- [ ] **Step 5: Commit.** Commit the three files as `fix(shell): validate dock API arguments`.

### Task 3: Complete persistence events and lock documentation

**Files:**
- Modify/Test: `crates/base/src/dock/dock_area.rs`
- Modify: `crates/shell/src/typings.rs`
- Modify: `website/shell/dock.md`
- Modify: `website/zh-CN/shell/dock.md`
- Test: `crates/shell/src/tests/dock.rs`

**Interfaces:**
- Produces: exactly one `LayoutChanged` event for an effective size change.
- Consumes: existing Dock event subscriptions and rearrangement-only lock behavior.

- [ ] **Step 1: Write failing tests.** Subscribe to a real area, change size and expect one event; repeat the same value and expect no second event. Add a JS test whose subscriber observes the updated size in `dump()`.
- [ ] **Step 2: Verify RED.** Run `cargo test -p gpui-base dock_size_change_emits_one_layout_event` and `cargo test -p gpui-shell programmatic_dock_size_change_reaches_persistence_subscriber`.
- [ ] **Step 3: Implement the event.** Compare effective old/new values and call both `notify` and `emit(LayoutChanged)` only when changed.
- [ ] **Step 4: Fix the contract text.** State in typings and both website languages that locking prevents rearrangement/drop but retains dock/tile resize. Change only the relevant comment in the already-dirty typings file.
- [ ] **Step 5: Verify GREEN.** Run the two focused tests, `cargo test -p gpui-shell typings::tests`, and inspect all lock text with `rg -n "locked|锁定"`.
- [ ] **Step 6: Commit.** Commit these files as `fix(dock): publish programmatic size changes`.

### Task 4: Cache replayable Dock chrome specs

**Files:**
- Modify: `crates/shell/src/dock.rs`
- Modify: `crates/shell/src/engine/quickjs/dock_api.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Modify: `crates/shell/src/metrics.rs`
- Test: `crates/shell/src/tests/dock.rs`

**Interfaces:**
- Produces: bounded `ChromeSpecCache` keyed by hook/container with `{callback, payload, arena, root}` and a `frame_script_calls` metric.
- Consumes: `CallbackId`, `SpecArena`, `SpecId`, `materialize_subtree`, `DockChromeSlots`, container ids/placements, `ContentSlot`.

- [ ] **Step 1: Write failing cache tests.** Count a JS `tab_bar` handler: unchanged draws stay at one, payload change increments, and a new callback increments. Verify cached `dock_content()` renders current native content.
- [ ] **Step 2: Count the frame's script calls.** Increment `frame_script_calls` in `time_frame_script`; draw a representative clean dock repeatedly and assert in `tests::dock` that the count does not change after its first frame.
- [ ] **Step 3: Verify RED.** Run the three new focused cache/content/benchmark tests and confirm counts currently grow per draw.
- [ ] **Step 4: Implement the cache.** Store it on `ScriptChrome`; key every hook by stable native identity. Hit only when callback and JSON payload match. On miss, call JS to produce arena/root, replace the entry, then materialize. Hold the whole cache under a hard entry bound and never cache errors; a successful null is an empty description and is cached.
- [ ] **Step 5: Preserve native frame state.** Scope `ContentSlot::install` around materialization on hits and misses; keep command lookup against current-frame `DockContexts`.
- [ ] **Step 6: Verify GREEN.** Run `cargo test -p gpui-shell tests::dock`, the new benchmark test, and `cargo test -p gpui-shell tests::snapshot`.
- [ ] **Step 7: Commit.** Commit the six files as `perf(shell): cache dock chrome descriptions`.

### Task 5: Verify and deliver

**Files:**
- Modify only files needed for failures caused by Tasks 1-4.

**Interfaces:**
- Consumes: Tasks 1-4.
- Produces: formatted, reviewed, committed, pushed branch.

- [ ] **Step 1: Format/check.** Run `cargo fmt --all -- --check`, `git diff --check`, and `git status --short`; confirm no `.claude` files or unrelated changes are staged.
- [ ] **Step 2: Run suites.** Run `cargo test -p gpui-base dock`, `cargo test -p gpui-shell --lib`, and `cargo test -p gpui-shell --release --lib benchmark -- --nocapture`.
- [ ] **Step 3: Audit the design.** Confirm all five spec requirements have direct code and test evidence; no unchecked numeric narrowing remains; every entity insertion is fallible; unchanged chrome hits do not call `with_js`.
- [ ] **Step 4: Commit formatting if needed.** Commit only actual formatter output as `chore: format shell dock fixes`; skip if empty.
- [ ] **Step 5: Push.** Run `git push origin shell-keyboard-pointer-actions-window` and confirm the remote reaches local HEAD.
