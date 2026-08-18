# Text Selection Showcase Layout

## Goal

Make drag-to-scroll selection easy to understand and reliable in the embedded
gpui-base showcase. The selectable document must scroll independently while
selection status and its clear action remain visible.

## Layout

The example uses one fixed-height column with two regions:

1. A flexible document viewport containing the selectable title and paragraphs.
   This is the only region connected to the selection `ScrollHandle`.
2. A non-scrolling footer containing the current selection status, copied-text
   preview, and Clear selection button.

The footer is not a selection participant and does not contribute document
order. Moving it outside the viewport prevents it from changing the scroll
extent or intercepting drag-to-scroll gestures near the document boundary.

The website gives gpui-base example iframes enough height to display both
regions without compressing the document viewport. Mobile keeps the same
structure with a smaller, viewport-conscious height.

## Behavior

- The document contains enough text to overflow its viewport.
- Dragging near the document viewport edges scrolls only the document.
- Selection status remains visible while the document scrolls.
- Clearing selection resets the status without changing the document position.
- The example keeps its existing borderless outer presentation.

## Verification

- A layout-focused test verifies that the footer is outside the scrollable
  document element.
- The existing selection and auto-scroll behavior remains covered by the base
  showcase and text-selection tests.
- Native and WASM examples compile.
- The website build succeeds at desktop and mobile CSS breakpoints.
