# TextView Shift+click Selection Design

Date: 2026-08-15

## Goal

Make selectable `TextView`s extend a selection from the most recent ordinary
click when the user Shift+clicks, matching AppKit and the repository's editable
Input/Editor controls.

## Interaction semantics

- An ordinary single click begins a selection with coincident `anchor` and
  `cursor` endpoints, so it has no visible highlight but establishes the next
  extension anchor.
- Shift+click preserves a usable existing `anchor` and moves only `cursor`.
- Repeated Shift+click and Shift+drag keep the same anchor. Crossing it reverses
  the visual direction without replacing it.
- If no usable anchor exists, Shift+click falls back to an ordinary click.
- A later ordinary click clears the old range and establishes a new anchor.
- Double/triple click, modal scope confinement, proxy endpoints, focus, and
  content-anchored scrolling keep their existing behavior.
- A mouse gesture owned by a suppressing component such as Input or Button does
  not begin or extend TextView selection and must not leave a stale highlight.

## Architecture

Keep `WindowTextSelection.anchor` and `cursor` as the only durable endpoint
state. Add a small begin/extend mode to the controller-to-Root selection-start
operation. Begin assigns both endpoints. Extend restores the resolvable anchor
staged for the current mouse event, assigns only the cursor, and otherwise falls
back to Begin behavior.

Mouse capture continues to reset the per-event suppression flag and clear the
old selection. Immediately before clearing, a Shift+click temporarily stages the
old anchor in `pending_extension_anchor`; an unsuppressed bubble handler consumes
it. This event-local staging is cleared by normal selection clearing and is not
a parallel click-history model. Keeping the clear in capture means an Input,
Button, or other component that stops bubble propagation cannot leave stale
TextView highlights.

Both modes set `is_selecting`, so the existing mouse-move path continues to move
only the cursor. Endpoint resolution, painting, copying, and autoscroll remain
unchanged.

## Testing

Add focused visual tests in `crates/ui/src/text/window_selection.rs` for:

- ordinary click followed by Shift+click in one TextView;
- repeated Shift+click with a stable anchor, including crossing the anchor;
- Shift+mousedown followed by drag;
- a later ordinary click resetting the anchor;
- Shift+click without a prior anchor;
- extension across TextViews;
- Shift+click on the suppressing test control clearing stale selection.

Run the focused tests first for the red/green cycle, then the complete
`window_selection` test target and formatting checks.

## Evidence

The supporting Apple, local Input/Editor, and pinned Zed sources are recorded in
[`docs/research/2026-08-15-textview-shift-click-selection.md`](../../research/2026-08-15-textview-shift-click-selection.md).
