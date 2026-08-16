# Final architecture review fixes

## Outcome

The final text-selection architecture findings are addressed in `gpui-base` and
the `gpui-component` adapter. The public seam now consists of opaque selection
data with builders/readers, a renderer-neutral selection handle, and documented
window scope/registration methods. Window selection state is retained by the
`TextSelection` element; the application registry is only a weak locator.

## Changes

- Made `TextSelectionHandleState` private, removed its re-export and
  `TextSelectionHandle::state()`, and moved snapshot, projection, callback,
  local-selection, and copy configuration onto `TextSelectionHandle` methods.
- Made `TextSelection::scope`, its window-level associated functions, and
  `TextSelectionHandle::register` part of the documented custom-renderer interface. The deprecated
  component bridge remains under the hidden `gpui_base::__private` namespace.
- Privatized every field on `TextSelectionEndpoint`, `TextSelectionSnapshot`,
  `TextSelectionRegistration`, `TextSelectionRun`, and `TextSelectionRunState`; added
  minimal constructors, builders, and readers; migrated the UI adapter,
  example, implementation, and tests.
- Split copy into a lease-bound collection phase and a callback-resolution
  phase. Renderer copy callbacks run only after both the window-state and
  handle-state guards have been dropped and receive `&mut App` for legitimate
  reentrant selection operations.
- Replaced the strong application-global window registry with weak locators.
  GPUI retained element state holds the strong entity, release cleanup clears
  handle-local state and removes the matching weak locator, and event/frame
  closures capture weak entities. The zero-sized `TextSelection` element binds
  the opaque state during prepaint; same-render scope changes are retained until
  that binding occurs, and `Root` mounts the element first.
- Added `document_order` as the stable tie-break for equally sized hovered
  participants.
- Derived `Default` where appropriate and resolved strict Clippy findings,
  including needless dereference and cloned single-item slice patterns.

## TDD evidence

Each behavioral/API change started with a focused failing test or compile
failure, then passed after the implementation:

- `public_selection_data_uses_builders_and_readers`
- `selection_handle_is_the_public_adapter_seam`
- `copy_callback_can_reenter_window_and_handle_selection`
- `equal_area_hovered_regions_break_ties_by_document_order`
- `two_windows_isolate_selection_copy_clear_and_release_ownership`
- `unit_selection_element_supports_scope_and_registration_on_the_first_frame`

The ownership regression covers independent selection, copy, and clear for two
windows, closes one window, verifies the other remains usable, and verifies the
closed window's weak locator is pruned. Existing lifecycle, duplicate-element,
scope, virtualization, and component compatibility tests remain in the full
suite.

## Two-axis review

The Standards review found one keyed-state documentation gap: the stable
`"window-text-selection"` identity and one-first-child lifecycle were not part
of the public contract. The documentation now states both; follow-up review
confirmed the finding is resolved. It retained one low-severity judgement call
about the similar public/prebound `Element` implementations, which share state
retention and paint helpers but keep distinct acquisition paths.

The Spec review found a first-frame scope-registration gap. The public unit
`TextSelection` element now consumes pending same-render scope state during its
first prepaint, and a RED/GREEN test covers same-render scope plus first-draw
registration. No other Standards or Spec findings remained.

## Verification

- `cargo test --workspace --all-targets` — passed (including 340 `gpui-base`
  unit tests, 427 `gpui-component` unit tests, compatibility suites, and the
  `selectable_text` example test).
- `cargo clippy --workspace --all-targets -- --deny warnings` — passed.
- `cargo check --workspace --all-targets` — passed.
- `cargo build -p gpui-base --example selectable_text` — passed.
- `cargo test -p gpui-base --example selectable_text` — passed (1 test).
- `cargo fmt --all -- --check` — passed.
- `git diff --check` — passed.

## Deliberate non-change

`TextViewStyle::PartialEq` still evaluates heading callbacks for levels 1–6.
A stored callback fingerprint is not safe with the current public mutable
`heading_font_size` and `heading_font_size` callback fields: either field can be
changed after construction, making a cached fingerprint stale. Pointer identity
would also change the established semantic equality of independently created
equivalent callbacks and cause unnecessary layout invalidation. The callback
comparison therefore remains unchanged pending a separately designed immutable
style seam.

## Scope guard

`.github/workflows/release.yml` was not modified by this work.
