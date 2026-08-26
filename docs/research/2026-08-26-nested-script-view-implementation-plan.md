# Nested Script Views Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add independently invalidated nested `ScriptView` entities and move the Longbridge price chart into one, so pointer movement rebuilds only the chart instead of the entire application.

**Architecture:** A root script creates retained child-view handles outside render. Each child owns its GPUI entity, script object, render snapshot, callback generation, and invalidation boundary. The parent passes data through an explicit update operation and mounts the child through a component node. Child input callbacks notify only the child entity.

**Tech Stack:** Rust, GPUI entities, QuickJS/rquickjs, gpui-shell materialization, JavaScript application code, Rust and JavaScript tests.

**Spec:** `docs/research/2026-08-26-nested-script-view-design.md`

## Global constraints

- [ ] Work in the current checkout; do not create a worktree.
- [ ] Do not create or commit anything under `docs/superpowers`.
- [ ] Do not throttle or silently discard shell pointer events.
- [ ] Give every child an independent snapshot and callback generation.
- [ ] Reject child creation and prop updates during render or layout.
- [ ] Remove the temporary Longbridge root-level mouse-move coalescer after migration.

## Task 1: Make hover lifecycle safe across snapshot replacement

**Files:**

- Modify: `crates/shell/src/materialize.rs`
- Modify: `crates/shell/src/tests/render.rs`

- [ ] Run `cargo test -p gpui-shell --lib a_mouse_move_rebuild_keeps_later_callbacks_live -- --nocapture` and confirm the extended hover assertion fails before the fix.
- [ ] Preserve the element bounds captured during prepaint in the hover callback.
- [ ] Ignore a stale snapshot's `on_hover(false)` when the current mouse position remains inside those bounds.
- [ ] Continue dispatching genuine pointer exits.
- [ ] Re-run the focused test and the related pointer-handler tests.

## Task 2: Retain nested ScriptView entities safely

**Files:**

- Modify: `crates/shell/src/entities.rs`
- Modify: `crates/shell/src/view.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Modify: shell entity/view tests as needed

- [ ] Add a retained entity-store variant for `Entity<ScriptView>` and typed lookup/release helpers.
- [ ] Distinguish application-root ownership from nested-view ownership in `ScriptView`.
- [ ] Ensure dropping a nested view releases only its own snapshots and entity-owned resources; it must not call application-wide entity or task cleanup.
- [ ] Keep application-wide cleanup on the root view so siblings cannot be released when one child is dropped.
- [ ] Reuse the existing view-construction path while allowing a child constructor/init to receive initial props.
- [ ] Add tests proving that releasing one child preserves its sibling and that dropping a child does not release application-owned state.

## Task 3: Expose ViewHandle and child_view

**Files:**

- Modify: `crates/shell/src/spec.rs`
- Modify: `crates/shell/src/materialize.rs`
- Modify: `crates/shell/src/engine/quickjs/mod.rs`
- Modify: `crates/shell/src/typings.rs`
- Modify: `crates/shell/src/tests/render.rs`
- Modify: snapshot/API contract tests as needed

- [ ] Add failing tests for `ViewHandle.new(Class, props)`, `handle.set_props(props)`, and `child_view(handle)`.
- [ ] Add failing tests for child creation/update during render, released handles, duplicate mounting in one snapshot, and child notification isolation.
- [ ] Add a child-view component/spec node that materializes to the retained `Entity<ScriptView>`.
- [ ] Construct the child script object under the child's own entity/event scope and retain the originating module lease and application identity.
- [ ] Implement `set_props` by invoking optional `update(props)` under the child event scope, then refreshing only the child.
- [ ] Register callbacks rendered by a child against that child's entity and callback generation.
- [ ] Reject mutation from render/layout with clear errors and reject mounting a handle more than once in one snapshot.
- [ ] Add TypeScript declarations and public API exports.
- [ ] Run focused API tests, then `cargo test -p gpui-shell --lib`.

## Task 4: Migrate the Longbridge price chart

**Files:**

- Create: `/home/jason/work/gpui-shell-longbridge/app/price_chart_view.js`
- Modify: `/home/jason/work/gpui-shell-longbridge/app/main.js`
- Modify: `/home/jason/work/gpui-shell-longbridge/app/ui.js`
- Modify: `/home/jason/work/gpui-shell-longbridge/app/chart.js`
- Modify: `/home/jason/work/gpui-shell-longbridge/app/chart.test.js`
- Modify: `/home/jason/work/gpui-shell-longbridge/tests/app_vectors.rs`

- [ ] Add a failing test proving laid-out five-day points retain their trading date.
- [ ] Add an application contract test proving the root mounts a retained chart child.
- [ ] Move chart geometry, hover state, mouse handlers, indicator, and tooltip rendering into `PriceChartView`.
- [ ] Create the chart handle during root initialization and push new chart props only when selected symbol, series, theme, or layout inputs change.
- [ ] Keep quote updates that do not alter chart props out of the child update path.
- [ ] Remove `chartPointer`, `chartHoverFramePending`, the 16 ms timer, and the root chart mouse handlers.
- [ ] Preserve the indicator interaction and fix the tooltip date so it never renders `undefined`.
- [ ] Run the JavaScript chart tests and Longbridge application contract tests.

## Task 5: Verify release performance and lifecycle behavior

**Files:**

- Modify performance documentation only if new measurements replace or clarify existing measured data.

- [ ] Build gpui-shell and Longbridge in release mode.
- [ ] Verify visually that the chart indicator follows the pointer and clears on a genuine exit.
- [ ] Measure idle, quote-update, and chart-hover render counters with the existing profiler.
- [ ] Confirm pointer movement increments the child render counter without incrementing the parent render counter.
- [ ] Confirm the event loop has no queued callback growth after a sustained pointer sweep.
- [ ] Run `cargo fmt --all -- --check` in gpui-component.
- [ ] Run `cargo test -p gpui-shell --lib` in gpui-component.
- [ ] Run Longbridge JavaScript tests and `cargo test --release --test app_vectors --test application_contract`.
- [ ] Run `git diff --check` in both repositories and inspect both working trees for unrelated or generated files.
