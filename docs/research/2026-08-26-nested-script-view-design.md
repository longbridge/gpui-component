# Nested Script Views

## Problem

`gpui-shell` currently mounts one `ScriptView` for an application root. An event callback's
`cx.notify()` invalidates that whole view. This is correct for ordinary application changes, but
not for high-frequency local interaction: the Longbridge price-chart indicator caused the root
view to rebuild in 12–13 ms for each pointer sample. A fast pointer produced 203 script renders in
one second, queued work faster than the UI thread could consume it, and made unrelated controls
appear frozen.

Coalescing pointer events prevents an unbounded queue but still rebuilds the watchlist and stock
details for a change confined to the chart. The runtime needs a smaller invalidation boundary.

## Decision

Add retained nested script views. A nested view is an ordinary `ScriptView` entity with its own
script object, current and previous snapshots, dirty flag, error surface, callback generations and
task ownership. Its render protocol is unchanged; only its ownership and script-facing mounting
API are new.

This is not a retained element tree or a reconciler. The parent snapshot contains one host element
that mounts an already-created child entity. GPUI still rebuilds elements from each view's cached
description on every repaint.

## Script API

```js
import { View, ViewHandle, child_view } from "gpui";

class PriceChart extends View {
  init(props) {
    this.props = props;
    this.hovered = null;
  }

  update(props) {
    this.props = props;
  }

  render(cx) {
    // Pointer callbacks registered here call cx.notify(), which invalidates
    // this PriceChart entity rather than its parent application view.
  }
}

class Workspace extends View {
  init() {
    this.chart = ViewHandle.new(PriceChart, { points: [] });
  }

  receiveHistory(points) {
    this.chart.set_props({ points });
  }

  render() {
    return this.chart;
  }
}
```

`ViewHandle.new(Class, props?)` is allowed only in `init`, an event handler or a task, where a live
window and application context exist. It constructs the script object, creates the final GPUI
entity before calling `init(props, cx)`, and registers the entity in the application-owned entity
store. Creation during `render` or the layout phase is rejected.

`handle.set_props(props)` runs the child's optional `update(props)` under the child's event scope,
then invalidates and notifies only that child. It is also forbidden during `render` and layout:
parent-to-child synchronization must happen where the data changes, not as a hidden render side
effect. Values remain inside the same QuickJS runtime; the host does not serialize them.

When a child reads shared state and needs no new props, its parent calls `cx.notify(handle)` instead.
This mirrors GPUI's targeted entity notification: it invalidates and notifies the child without
calling `update(props)` or creating the update transaction's reachable-object checkpoint.

A handle is a child wherever a child is taken — `.child(handle)`, or returned from `render` —
exactly as an `Entity<V>` is renderable in GPUI. Mounting produces a single-use description of the
retained child entity, so the same handle may appear once in a parent snapshot. Reusing it twice in
one description is an error because GPUI cannot mount one entity at two positions.

`handle.release()` removes the script's ownership. A mounted GPUI entity may finish its current
frame, but the handle can no longer be updated or mounted again.

## Ownership and lifecycle

- A child inherits its parent's `Policy`, application generation and module lease.
- Child `init`, `update`, render callbacks, timers and async continuations enter scopes with the
  child entity as the current view. Their `cx.notify()` therefore targets the child.
- The entity store owns the child handle and releases all children belonging to an application
  when that application unloads.
- The parent may drop or replace a snapshot without retiring the child's callback generations;
  those generations belong to the child snapshots.
- Dropping or releasing a child retires its current and previous snapshots before the runtime, then
  cancels child-owned tasks. Existing `ScriptView` field-order guarantees remain load-bearing.
- Hot reload replaces the root object in place. Re-running root `init` creates the replacement
  children; releasing the old root releases its application-owned children and their snapshots.
  Preserving a nested entity across a source-generation change is deliberately not required.

## Rendering and errors

The child materializes through GPUI's ordinary entity rendering. A failed child render keeps its
last good child snapshot and draws the existing error banner inside the child's bounds. It does not
replace or invalidate the parent snapshot.

Pointer movement still enters JavaScript when the application explicitly registers
`on_mouse_move`; gpui-shell does not silently throttle GPUI events. The performance gain comes from
describing only the chart subtree. The Longbridge chart must remove its temporary 16 ms root-view
coalescing after it moves into the child view.

## Longbridge integration

The price chart becomes a `PriceChart` child view. History loads, selected-symbol changes and live
candle updates call `set_props` outside the parent render. Pointer position and hover indicator
state live entirely on the child instance. Moving the pointer rebuilds only the chart description;
Watchlist and Stock Details remain on their cached parent snapshot.

The tooltip displays the point's market-local date and time. The current `undefined` date is fixed
by retaining each prepared point's `date` when layout geometry is produced.

## Tests and acceptance criteria

1. A child event calling `cx.notify()` increments the child render count without incrementing its
   parent's render count.
2. Updating child props rebuilds the child once and leaves the parent snapshot unchanged.
3. Parent snapshot replacement keeps mounted child callbacks live.
4. Releasing or unloading retires child callbacks and tasks without leaking QuickJS persistent
   values.
5. Creation and `set_props` during render/layout fail at the call site with phase-specific errors.
6. Child render failures preserve the parent and the child's previous good snapshot.
7. Longbridge chart vectors cover nearest-point selection and preserved market-local dates.
8. In an active Release window, sweeping the chart must not increase the parent script-render
   counter. The chart child may render at pointer frequency without building an event backlog, and
   unrelated tabs and controls must remain responsive.
