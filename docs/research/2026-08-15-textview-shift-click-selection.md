# TextView Shift+click selection research

Date: 2026-08-15

## Decision

Implement `TextView` Shift+click as selection extension around a stable anchor:

- An ordinary single click establishes a new anchor and a coincident cursor.
- Shift+click keeps that anchor and moves only the cursor to the clicked endpoint.
- Repeated Shift+click keeps the same anchor. If the cursor crosses the anchor, the visual selection reverses direction; the anchor itself does not move.
- A Shift+mousedown followed by dragging begins by extending to the pressed endpoint, then continues moving that same cursor endpoint.
- A later ordinary click starts a new anchor.

This matches both this repository's editable Input/Editor engine and Zed's Editor. Apple explicitly documents Shift-click extension and subsequent Shift-clicks; the precise stable-character-anchor rule is an implementation conclusion from Apple's anchored/non-anchored selection model plus the two concrete editor implementations, rather than a sentence Apple states verbatim.

## Apple/AppKit evidence

Apple's [`NSTextView.selectionGranularity`](https://developer.apple.com/documentation/appkit/nstextview/selectiongranularity) documentation is the most direct behavioral source. It says selection granularity controls how a selection is modified when the user Shift-clicks or drags after a multiple click, and specifically says that **subsequent Shift-clicks** extend a word selection by words. This proves that Shift-click is an extension gesture, repeated Shift-click continues an existing selection, and the initial click count's character/word/line-like granularity survives into later extension gestures.

Apple's archived [Text Editing Programming Guide: Setting Focus and Selection Programmatically](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/TextEditing/Tasks/SetFocus.html) explains that `NSTextView` repeatedly calls `setSelectedRange(_:affinity:stillSelecting:)` while tracking a user's selection and that the initial click count determines granularity. This supports treating the down-and-drag sequence as updates to one selection rather than unrelated selections.

Apple's current [`NSTextSelection`](https://developer.apple.com/documentation/appkit/nstextselection) model describes affinity in terms of the direction of the selection's **non-anchored edge**, and describes `anchorPositionOffset` relative to the initial tap or click. The latter is a visual line-fragment offset, not itself a character-index selection anchor, so it should not be cited as direct proof of the exact character anchor. Together these APIs nevertheless establish Apple's anchored-edge/non-anchored-edge model.

What Apple documents directly:

- Shift-click modifies/extends a selection.
- Subsequent Shift-clicks keep extending it.
- Dragging and Shift-click extension retain the selection granularity established by the initial click count.
- A selection has anchored and non-anchored edges.

What is inferred rather than stated verbatim in Apple's prose:

- click A, Shift-click B, then Shift-click C keeps A as the exact character anchor;
- crossing A reverses direction without replacing A;
- Shift+drag uses precisely the same character anchor.

Those inferred details are strongly corroborated by the local Input/Editor implementation and Zed's Editor below.

## This repository: Input and Editor

`crates/ui/src/input/editor.rs` is only the styled wrapper. Its `RenderOnce` implementation renders the same `Input` element used by the other editable controls (currently lines 112–129). `crates/ui/src/input/state.rs` dispatches Input, Textarea, and Editor to their shared base engine (currently lines 29–148). The actual selection behavior is therefore in `crates/base/src/input/base/state.rs`.

The relevant flow is:

1. [`InputBaseState::on_mouse_down`](../../crates/base/src/input/base/state.rs) (currently lines 1619–1675) converts the pointer to an offset. On a plain click it calls `move_to(offset, None, cx)`; on Shift+click it calls `select_to(offset, cx)` without first resetting the selection.
2. [`InputBaseState::cursor`](../../crates/base/src/input/base/state.rs) (currently lines 1949–1962) defines the moving end as `selected_range.start` when `selection_reversed`, otherwise `selected_range.end`.
3. [`InputBaseState::select_to`](../../crates/base/src/input/base/state.rs) (currently lines 2098–2132) changes only that moving end. When it crosses the fixed end, it swaps the normalized range and toggles `selection_reversed`. This preserves the original anchor while allowing selection direction to flip.
4. [`InputBaseState::on_drag_move`](../../crates/base/src/input/base/state.rs) (currently lines 2291–2329) repeatedly calls the same `select_to`. Consequently Shift+mousedown first extends from the pre-existing anchor, and subsequent movement continues from that anchor.

No dedicated Shift+mouse regression test was found in `crates/base/src/input`; the conclusion above comes directly from the production control flow and state transition logic. This behavior applies equally to this repository's Input, Textarea, and Editor because all three dispatch to the shared engine.

## Current TextView behavior and gap

Window-level TextView selection is stored in `TextSelection` in [`crates/ui/src/text/window_selection.rs`](../../crates/ui/src/text/window_selection.rs) (currently lines 137–179). It already has exactly the needed representation: a separate `anchor`, `cursor`, and `is_selecting` flag. A zero-length click remains represented internally by coincident endpoints even though `resolved_points` returns no visible selection for equal points (currently lines 204–217).

The missing behavior is in the root mouse controller in the same file (currently lines 818–840):

- every left-button mouse-down clears the previous selection unconditionally during capture (lines 822–827), without looking at `event.modifiers.shift`;
- the bubble phase then calls `start_text_selection`, which assigns **both** `anchor` and `cursor` to the new endpoint (currently lines 426–457).

Thus a plain click already leaves the right latent anchor state, but the next Shift+click destroys it before it can be reused. The existing drag update path already moves only `cursor` (`update_text_selection`, currently lines 459–505), so it is compatible with stable-anchor Shift+drag once selection start distinguishes “begin” from “extend.”

Existing nearby coverage includes plain-click clearing (`mouse_down_clears_previous_selection`, currently lines 1480–1494) and double-click word selection (starting at line 1496), but no Shift+click or Shift+drag test was found.

## Upstream Zed Editor at this repository's pinned revision

`Cargo.lock` pins GPUI's Zed source to commit [`cc053a4a6fa2fd0e8793201ed9099466af1be0b1`](https://github.com/zed-industries/zed/tree/cc053a4a6fa2fd0e8793201ed9099466af1be0b1). The corresponding Editor implementation provides an independent production reference:

- In [`crates/editor/src/element/mouse.rs` lines 723–758](https://github.com/zed-industries/zed/blob/cc053a4a6fa2fd0e8793201ed9099466af1be0b1/crates/editor/src/element/mouse.rs#L723-L758), an unmodified click dispatches `SelectPhase::Begin`, while Shift-only click dispatches `SelectPhase::Extend`.
- [`SelectPhase`](https://github.com/zed-industries/zed/blob/cc053a4a6fa2fd0e8793201ed9099466af1be0b1/crates/editor/src/editor.rs#L408-L431) explicitly separates `Begin`, `Extend`, `Update`, and `End`.
- In [`extend_selection`, lines 1190–1253](https://github.com/zed-industries/zed/blob/cc053a4a6fa2fd0e8793201ed9099466af1be0b1/crates/editor/src/selection.rs#L1190-L1253), Zed saves the existing selection tail, begins at the clicked position, then expands the pending selection back to the saved tail and marks it as extending. It also carries forward character/word/line selection mode.
- Mouse movement dispatches `SelectPhase::Update` in [`mouse.rs` lines 1115–1124](https://github.com/zed-industries/zed/blob/cc053a4a6fa2fd0e8793201ed9099466af1be0b1/crates/editor/src/element/mouse.rs#L1115-L1124), so dragging updates the selection begun or extended on mouse-down.

Zed therefore confirms the same interaction architecture: ordinary click begins; Shift+click extends from the existing tail/anchor; movement updates that pending selection; click count/granularity is retained.

## Recommended TextView implementation boundary

Use the existing `TextSelection.anchor` and `.cursor`; do not add a parallel `last_click_endpoint`. The new behavior should be expressed as two start modes:

- **Begin:** clear old local/window selection, set `anchor = endpoint`, set `cursor = endpoint`.
- **Extend:** when a usable anchor exists, preserve it and set `cursor = endpoint`; otherwise fall back to Begin. Then set `is_selecting = true`, so a following drag continues to update only the cursor.

The root controller must also preserve existing suppression semantics. A Shift+click owned by Input, Button, or another suppressing component must not accidentally retain a stale TextView selection merely because clearing was skipped during capture. Therefore the implementation should decide clearing after it knows whether the gesture was suppressed, or explicitly clear on the suppressed Shift-click path. It should retain the existing rules for plain click, multiple-click inline selection, selection scopes/modal layers, focus, and blank-space proxy endpoints.

Minimum regression matrix:

1. click A, Shift+click B selects A–B;
2. click A, Shift+click B, Shift+click C keeps A fixed, both when C remains on the same side and crosses A;
3. click A, Shift+mousedown B, drag to C selects A–C;
4. a later plain click D clears the range and establishes D as the next anchor;
5. Shift+click without a usable prior anchor falls back to a plain click;
6. cross-TextView extension works with content-anchored endpoints;
7. Shift+click on a suppressing interactive control clears/does not extend the window TextView selection;
8. existing double/triple-click granularity and modal selection-scope behavior remain unchanged.
