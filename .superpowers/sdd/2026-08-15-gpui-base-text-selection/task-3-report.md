# Task 3 report — TextView adapter and window selection

## Status

Complete after independent-review fix round 1. `TextView` and plain selectable
regions share one implementation-private window state. The public integration
surface is a zero-sized `TextSelection` element plus `WindowTextSelection`;
there is no public state type or lifecycle setup call.

## Public interface

- A standard `Root` renders one `TextSelection` child. A custom root does the
  same; it does not create or store an entity.
- Renderers register frames with
  `window.register_text_selection_region(region, frame, cx)`.
- Scope, query, clear, and end operations use the same Window extension.
- Advanced TextView adapter code never sees the private `TextSelectionState`.
- Before the element paints, Window queries are safe empty/no-op operations.

## Review fixes and RED evidence

Each review finding was reproduced before implementation:

- Cross-region virtual TextViews omitted unpainted endpoint and middle blocks
  in both Plain and Source formats.
- A padded scrollable TextView treated wrapper bounds as its content origin.
- Replacement and same-size style reflow during/after a drag retained stale
  selection, while a compatible append lost its valid selection.
- Shift extension reused an anchor whose region had been swept.
- Base clear returned before TextView state cleared, and clear then select-all
  in one effect produced split state.
- A virtual-key callback re-entering window selection ran under the mutable
  state lease.
- The DnD guard had no discriminating test.
- The former public state seam forced Root, examples, and adapters to manage an
  entity contrary to repository element conventions.

## Implementation

- Added per-region coverage (`Bounded`, `FromStart`, `ToEnd`, `Full`) so
  TextView exports the correct virtual block interval across mixed regions.
- Separated the TextView hitbox from its `TextViewState::bounds` content frame.
- Added layout-breaking selection generations for replacement/style reflow,
  deferred invalidation across active drags, and append-compatible parser
  state. Synchronizing the background parser baseline also fixed append after
  a synchronous initial parse.
- Validated Shift anchors against current live registrations.
- Made clear and scope changes two-phase: mutate/cache-clear first, then invoke
  renderer callbacks outside the window-state lease.
- Resolved virtual keys in a second phase outside the window-state lease, with
  a reentrant regression test.
- Added an injectable active-DnD path and a test proving the cursor cannot move.
- Replaced the public state/controller seam with public `TextSelection` and a
  small `WindowTextSelection` interface; state lookup is private.

## Verification

```text
cargo test -p gpui-base text_selection::tests -- --nocapture
25 passed

cargo test -p gpui-component text::window_selection::tests -- --nocapture
46 passed

cargo test -p gpui-component
406 unit + 40 compatibility tests passed

cargo test -p gpui-base
330 unit + 1 integration test passed

cargo check --workspace --all-targets
passed

cargo fmt --all -- --check
passed

git diff --check
passed
```

Only the workspace's existing future-incompatibility warning for `block` and
`proc-macro-error2` was emitted.

## Commits

- `5414cb6ac13c6439c8f3e3dc4ae36a0e50399e06` — initial Task 3 migration
- `161ae096992371e9e6404f1dc2623fcf16e19a7e` — initial compatibility hardening
- Fix-round commit (this report) — encapsulate window selection state and
  address review findings

## Self-review and concerns

No blocking concerns. The state remains window-local through a private global
map because GPUI does not expose arbitrary Window-owned state; the element
fully encapsulates that implementation detail. Duplicate `TextSelection`
elements are harmless because event deduplication and frame sweeping use the
single private window state.

## Review fix round 2

- Renamed the public Window interface to `WindowTextSelection` and the private
  implementation to `TextSelectionState`.
- Expanded style equality to cover heading callbacks, code/table/table-cell
  refinements, inline code, and dark mode; every layout writes the complete
  style value.
- Scope and registration mutations lazily create private state before the
  element paints, while queries without an element remain no-op.
- Restored deprecated `Root::clear_text_selection` as a forwarding request
  without retaining an entity.
- Added ordered callback epochs so a deferred snapshot cannot overtake clear.
- Marked synchronous parse acknowledgements so they cannot clear a newer
  selection; coalesced appends still publish their parsed result.
- Deduplicated duplicate-element mouse-down preparation/begin handling so Shift
  anchors are not cleared twice.

## Review fix round 3

- Removed the second ordinary-click clear from bubble begin; capture produces
  the single ordered clear batch, dispatched after the state lease.
- Made deprecated `Root::clear_text_selection` synchronously forward by stored
  `WindowId`, without storing a selection-state entity.
- Added explicit element-enabled state: lazy scope/region mutation may prepare
  state, but query, clear, and end remain no-op until `TextSelection` renders.
- Replaced the baseline boolean with `ParseMode`; a pure baseline acknowledgement
  is ignored while a baseline coalesced with append becomes an applied update.
- Compared heading callbacks by their six layout outputs rather than allocation
  identity, avoiding per-render invalidation for newly-created equivalent
  closures while detecting semantic heading-size changes.
- Added synchronous deprecated-forwarding and lazy-registration/local-selection
  regressions; duplicate capture/bubble preparation remains event-idempotent.

## Review fix round 4

- Split queued parse mode from `ParsedUpdate` outcome metadata: full parse,
  baseline acknowledgement, and selection compatibility are independent.
  Full-baseline plus append is compatible and preserves a newer select-all.
- Replaced persistent element enablement with a paint heartbeat that expires
  when `TextSelection` disappears on the next frame.
- Removed the no-self bridge from public `WindowTextSelection`; deprecated Root
  forwarding uses a hidden free function keyed by `WindowId`.
- Added removal-heartbeat and forced full-parse/append/select-all regressions.
- Updated remaining selection comments and plan language to element/window-state
  terminology.

The user-owned `.github/workflows/release.yml` change was neither modified nor
staged.
