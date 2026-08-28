# Targeted Script-View Notification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose GPUI targeted entity notification through `cx.notify(entity)` without invoking nested-view update or checkpoint logic, then migrate Longbridge away from dummy revision props.

**Architecture:** Extend the existing QuickJS context `notify` member with an optional branded Entity argument. Resolve and authorize the retained view through the nested-view alias store, then use `ScriptView::refresh` so the script snapshot becomes dirty before GPUI notification. Keep `set_props` as the transactional data-delivery interface.

**Tech Stack:** Rust, rquickjs, GPUI, JavaScript, generated TypeScript declarations.

**Spec:** `docs/superpowers/specs/2026-08-28-targeted-view-notify-design.md`

## Global Constraints

- Preserve zero-argument `cx.notify()` behavior.
- Targeted notify must not call child `update` or checkpoint reachable JavaScript state.
- Reuse retained-view liveness and application provenance validation.
- Do not introduce script-facing `refresh` terminology.
- Do not use wall-clock thresholds in regression tests.

---

### Task 1: Targeted notification runtime

**Files:**
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Test: `crates/shell/src/tests/render.rs`

**Interfaces:**
- Consumes: script Entity wrappers branded by `__entity` and carrying `__handle`.
- Produces: `cx.notify(target?: Entity): void` runtime behavior.

- [ ] **Step 1: Write failing integration tests**

Add a parent/child fixture whose parent button changes shared state and calls `cx.notify(this.child)` twice. Give the child an `update` method that increments a counter. Assert after one draw that the child render count advances once, the parent render count does not advance, and the update counter stays zero. Add rejected-call cases for malformed, released and foreign Entity targets.

- [ ] **Step 2: Run the focused tests and verify RED**

Run `cargo test -p gpui-shell targeted_notify -- --nocapture`.

Expected: failure because the current `notify` host function accepts no target and continues to notify the current parent.

- [ ] **Step 3: Implement target resolution and notification**

Change `context_object`'s `notify` binding to accept `Option<Value>`. With no argument, retain the current-view path. With an argument, validate `__entity`, extract `__handle`, resolve it through `nested_view_handles`, verify active-scope provenance, and obtain the live `Entity<ScriptView>`. For either path call `view.update(app, |view, cx| view.refresh(cx))`. Do not enqueue `PendingNestedOperation::Update` and do not call `update_nested_view`.

- [ ] **Step 4: Run focused and shell tests**

Run `cargo test -p gpui-shell targeted_notify -- --nocapture` and `cargo test -p gpui-shell`.

Expected: all pass.

### Task 2: Public declarations and documentation

**Files:**
- Modify: `crates/shell/src/typings.rs`
- Modify: `docs/research/2026-08-26-nested-script-view-design.md`
- Test: `crates/shell/src/typings.rs`

**Interfaces:**
- Consumes: targeted notify runtime from Task 1.
- Produces: `Context.notify(target?: Entity): void` in generated declarations.

- [ ] **Step 1: Make the declaration test fail**

Add a focused test expecting `notify(target?: Entity): void;`. Run `cargo test -p gpui-shell targeted_notify_is_declared` and verify it fails against `notify(): void`.

- [ ] **Step 2: Update declarations and docs**

Document current-view notification, targeted retained-view notification, and the distinction from `Entity.set_props`. Update the nested-view design's claim that `set_props` is the only parent-to-child repaint mechanism.

- [ ] **Step 3: Verify generated typings**

Run `cargo test -p gpui-shell typings -- --nocapture`.

Expected: all typing tests pass.

### Task 3: Longbridge migration and regression verification

**Files:**
- Modify: `../gpui-shell-longbridge/app/main.js`
- Modify: `../gpui-shell-longbridge/tests/application_contract.rs`
- Test: `../gpui-shell-longbridge/tests/app_vectors.rs`

**Interfaces:**
- Consumes: `cx.notify(Entity)` from Tasks 1–2.
- Produces: pane invalidation without revision props or nested-view update.

- [ ] **Step 1: Change the application contract test to require targeted notify**

Require `cx.notify(this.watchlistPanel)` and `cx.notify(this.detailPanel)`, and reject `workspaceRevision` plus `{ revision: ... }`. Run the focused contract test and verify RED.

- [ ] **Step 2: Migrate `syncWorkspacePanels`**

Give `syncWorkspacePanels` the active context parameter, replace both `set_props(props)` calls with `cx.notify(panel)`, update callers, remove `workspaceRevision`, and revise comments to describe targeted notification. Keep chart `set_props` calls unchanged because they deliver real props.

- [ ] **Step 3: Run all verification gates**

Run `cargo test -p gpui-shell`, then from Longbridge run `cargo test --all-targets`, followed by `git diff --check` in both repositories.

Expected: all tests pass and no whitespace errors remain.
