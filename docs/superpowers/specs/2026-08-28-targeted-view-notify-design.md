# Targeted Script-View Notification

## Problem

`gpui-shell` exposes `cx.notify()` only for the script view that owns the current
callback. Retained nested views sometimes read shared state instead of receiving
new props, so their parent needs to invalidate them without running their
`update(props)` method.

Today the only parent-to-child operation is `entity.set_props(props)`. Callers
therefore send dummy props, such as a monotonically increasing `revision`, only
to rebuild the child. That operation has materially different semantics: it
runs arbitrary child JavaScript inside a rollback transaction. The rollback
checkpoints every ordinary object reachable from the child. A child holding its
application model can consequently walk quotes, holdings and thousands of
candles on every repaint even when its `update` ignores the dummy prop.

This hidden cost caused pointer feedback in the Longbridge Watchlist to lag.
The hover style was native GPUI work, but quote-driven child updates occupied
the UI thread before GPUI could paint it.

## Decision

Extend the existing context notification interface with an optional retained
view target:

```ts
interface Context {
  notify(target?: Entity): void;
}
```

This mirrors GPUI's existing model:

```rust
Context<T>::notify()       // notify the context's entity
App::notify(EntityId)      // notify a specified entity
```

No script-facing `refresh` concept is added. `ScriptView::refresh` may remain
an implementation helper that combines snapshot invalidation with GPUI
notification.

## Semantics

`cx.notify()` preserves its current behavior. It marks the current
`ScriptView` snapshot dirty and asks GPUI to notify that view's entity.

`cx.notify(target)` resolves `target` to its retained `ScriptView`, marks that
child snapshot dirty and asks GPUI to notify the child entity. It does not:

- call the child's `update(props)`;
- create a JavaScript state checkpoint;
- create entity or task rollback checkpoints;
- manufacture or compare props;
- rebuild the parent snapshot.

Notification remains scheduled and coalesced by GPUI. Multiple notifications
of the same view before the next frame may produce one child snapshot rebuild.
The shell must not add a parallel notification queue.

`entity.set_props(props)` remains the interface for actual parent-to-child data
delivery. It continues to run the optional child `update(props)` transactionally
and then invalidates the child.

## Validation and errors

Targeted notify has the same phase rule as current-view notify. It is legal
from initialization, event and task scopes accepted by `ScopePhase::allows_notify`.
It throws synchronously during render, layout or any scope where notify is
already forbidden. The error names the written call, `cx.notify(entity)`.

The target must be a live retained script-view Entity owned by the current
application generation and policy. A released Entity, a stale Entity from a
reloaded application, a native retained-state handle, or a forged/non-Entity
object throws synchronously. Validation must reuse nested-view provenance and
liveness checks rather than relying only on the script-visible `__entity`
marker or numeric handle.

Calling `cx.notify(target)` when `target` is the current view is legal and has
the same result as `cx.notify()`.

As with current `cx.notify()`, a stale captured Context fails its existing
context-generation check before it can notify either target.

## Runtime flow

The context binding accepts an optional script value. With no value it follows
the current-view path unchanged. With a value it extracts the retained view
token, resolves the token through `nested_view_handles`, verifies provenance
against the active scope, and obtains the corresponding `Entity<ScriptView>`.

The final operation is the same for both paths:

```rust
view.update(app, |view, cx| view.refresh(cx));
```

`ScriptView::refresh` is required instead of a bare `App::notify(entity_id)`.
GPUI notification invalidates windows and notifies observers, but the shell's
cached script description is separately guarded by `ScriptView::dirty`. Without
setting that flag, GPUI would repaint the previous snapshot without entering
the child's JavaScript `render()`.

No mutation is queued in `PendingNestedOperation`: notification does not alter
child state and does not need the nested-update transaction or its ordering.

## Typings and documentation

The generated `gpui` declaration changes `Context.notify()` to:

```ts
notify(target?: Entity): void;
```

Its documentation distinguishes notification from prop delivery:

- use `cx.notify()` after changing state read by the current view;
- use `cx.notify(entity)` after changing shared state read by a retained child;
- use `entity.set_props(props)` when the child must receive and process new
  props.

The nested-view documentation must stop describing `set_props` as the only way
a parent can repaint a child.

## Longbridge migration

The Longbridge workspace panels retain the application once during `init` and
render shared application state. `syncWorkspacePanels` will stop sending dummy
revision props:

```js
if (panes & PANE_WATCHLIST) cx.notify(this.watchlistPanel);
if (panes & PANE_DETAIL) cx.notify(this.detailPanel);
```

The `workspaceRevision` field and revision prop object are removed. The panel
classes continue to have no `update` method because no props change after
construction.

Chart views still use `set_props`: symbol, series, state, layout and theme are
real child inputs processed by `PriceChartView.update`.

## Tests and acceptance criteria

Shell tests must prove:

1. `cx.notify()` still rebuilds only the current script view.
2. `cx.notify(child)` rebuilds the child exactly once without rebuilding the
   parent.
3. A child with an `update` method is notified without invoking that method.
4. Targeted notification performs no view-state checkpoint. This must be
   asserted through an observable checkpoint counter or a deliberately
   uncheckpointable reachable property, not a wall-clock threshold.
5. Repeated targeted notifications before drawing coalesce into one child
   render.
6. A released, stale, foreign, native-state or malformed target throws at the
   call site and does not notify another entity.
7. Render and layout calls fail with the existing phase-specific behavior.
8. Generated declarations contain `notify(target?: Entity): void` and remain
   synchronized with the checked-in declaration fixture.

Longbridge tests must prove:

1. Workspace repaint code uses targeted notify and carries no dummy revision.
2. Quote-driven pane repaint still rebuilds the intended child without
   rebuilding the retained chart unnecessarily.
3. Watchlist selection and chart publication behavior remain unchanged.

The full gpui-shell and Longbridge test suites must pass. No test may use a
wall-clock latency threshold; the regression signal is the absence of the
expensive checkpoint/update path.

## Out of scope

- Changing transactional `set_props` rollback semantics.
- Adding checkpoint timing metrics or slow-checkpoint warnings.
- Coalescing or dropping actual `set_props` calls.
- Adding priorities or preemption to GPUI event dispatch.
- Changing chart publication frequency or quote batching.

Those may be designed separately after targeted notification removes the
misuse that exposed this performance regression.
