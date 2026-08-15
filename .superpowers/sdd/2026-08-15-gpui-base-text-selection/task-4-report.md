# Task 4 report — scope migration and deprecated forwarding

## Status

Implementation complete; manual and performance verification remain for the
root agent. `gpui-component::Root` now assigns opaque base scope IDs to active
Dialog and Sheet state, modal surfaces use the base-owned scope marker, and the
component compatibility methods synchronously forward to the one base-owned
window selection state.

`crates/ui/src/text/window_selection.rs` contains integration tests only and is
compiled only under `cfg(test)`.

## TDD evidence

- Baseline: the existing component selection suite passed 47 tests.
- RED 1: modal scope tests were changed to consume the planned base
  `TextSelection::scope` seam. Compilation failed with `E0599` because that
  associated function did not exist.
- GREEN 1: after adding the base scope marker and registration override, the
  suite passed 48 tests, including the new component/base forwarding test.
- RED 2: the first production migration wrapped the complete `RenderOnce`
  Dialog. `drag_inside_dialog_still_selects_its_text` then failed (47 passed,
  1 failed), proving the generated popup was outside the effective marker
  paint stack.
- GREEN 2: Root now injects its opaque ID into Dialog/Sheet, whose actual
  selectable surfaces use the base marker. The focused suite passed 48 tests.
- RED 3: removing the real Sheet surface marker made
  `drag_inside_sheet_still_selects_its_text` fail with an empty selection.
- RED 4: making deprecated component `end_text_selection` a no-op let a later
  pointer move expand `"Hello "` into both TextViews, so the forwarding test
  failed.
- GREEN 3: restoring both forwarding paths produced a 50-test focused pass.
- RED 5: a scoped subtree panic left `SelectionScopeId(41)` at the top of the
  scope stack.
- GREEN 4: scoped rendering now pops before resuming the panic; the cleanup
  regression test passes.
- RED 6: the double-window reentry test could not compile against the
  App-global stack API because scope lookup had no window key.
- GREEN 5: window-keyed stacks isolate reentrant rendering; both the active
  window and unrelated-window assertions pass.

## Implementation

- Added private base `TextSelectionScopeStacks` and
  `TextSelection::scope(scope, element)`. Region registration automatically
  adopts the innermost marked scope during request-layout, prepaint, or paint.
- Scope stacks are keyed by window, so a reentrant render in another window
  cannot inherit the active window's scope. Scoped rendering also cleans up
  before resuming a subtree panic.
- Root allocates stable non-default IDs for each opened modal, stores them on
  `ActiveDialog`/`ActiveSheet`, selects the top Dialog before the Sheet, and
  injects the IDs into the rendered modal surfaces.
- Removed the component `SelectionScope` enum, component scope marker, and UI
  scope stack. TextView registration no longer reads UI-owned scope state.
- Root renders `gpui_base::TextSelection` automatically and uses fully
  qualified `WindowTextSelection` calls internally.
- Marked all four component `WindowExt` selection methods and
  `Root::clear_text_selection` deprecated with migration notes. Both legacy
  surfaces forward synchronously to base; clearing through either interface is
  observed immediately by the other.
- Updated internal TextView/link/copy call sites to use fully qualified base
  operations, avoiding extension-trait ambiguity during the deprecation
  window.

## Single-state and naming audit

- Searches find the authoritative `SelectionEndpoint` anchor/cursor fields and
  the window selection `HashMap` only in `crates/base/src/text_selection.rs`.
- There is no Root selection entity, selectable-view map, inline-bounds map, or
  component production window-selection engine.
- TextView's virtual-block adapter retains only a derived immutable snapshot
  projection for source export; it does not coordinate gestures or own a
  selection session.
- Production searches find no `TextSelectionHost`, `TextSelectionController`,
  `WindowTextSelectionExt`, or text-selection install API. The public lifecycle
  and window-operation names remain `TextSelection` and
  `WindowTextSelection`.

## Verification

```text
cargo test -p gpui-base text_selection --lib
32 passed

cargo test -p gpui-component text::window_selection::tests --lib
50 passed

cargo test -p gpui-base --lib
334 passed

cargo test -p gpui-component --lib
409 passed

cargo check --workspace
passed

cargo fmt --all -- --check
passed

git diff --check
passed
```

Only the workspace's existing future-incompatibility warning for `block` and
`proc-macro-error2` was emitted.

## Manual and performance verification

Pending root-agent verification; this subtask does not claim either result:

- Manual interaction verification for selection across ordinary content,
  Dialog, and Sheet surfaces.
- Performance comparison for selection rendering and region registration.

## Self-review and concerns

The standards review found two low-severity readability smells in the new base
marker: a collection name that did not state its stack semantics and repeated
push/delegate/pop code. These were addressed with
`TextSelectionScopeStacks` and one scoped helper, then the focused base and UI
tests were rerun.

The spec review found that the synthetic Sheet fixture did not exercise
`Sheet::render`, and that component `end_text_selection` lacked a discriminating
forwarding test. Both were added and independently shown to fail when their
production forwarding was temporarily removed.

No code or automated-test blockers. Manual and performance verification are
explicitly pending the root agent. The task commit is the commit containing
this report.
The user's pre-existing `.github/workflows/release.yml` change was neither
modified nor staged.
