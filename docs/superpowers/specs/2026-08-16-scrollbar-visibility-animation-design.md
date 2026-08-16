# Scrollbar Visibility Animation Design

## Goal

Make scrollbar visibility changes feel deliberate instead of popping on and
off. A vertical scrollbar slides inward from the right while fading in and
returns to the right while fading out. A horizontal scrollbar uses the same
edge-relative motion, entering upward from the bottom and exiting downward.

## Scope

The change applies to the shared `gpui_base::Scrollbar`, including scrollbars
used through `gpui_component` adapters. It covers `Scrolling` and `Hover`
visibility transitions. `Always` mode remains fully visible and stationary.

The animation affects painted scrollbar content only. Layout bounds, viewport
geometry, and interaction hitboxes remain fixed so the animation cannot cause
reflow or make pointer targeting follow a moving control.

## Motion

Scrollbar painting has opacity and position progress values from `0.0`
(hidden) to `1.0` (fully shown). Translation is one resolved track width at
position `0.0` and zero at `1.0`:

- Vertical scrollbar: translate right when hidden and move left into place.
- Horizontal scrollbar: translate down when hidden and move up into place.

Entrance lasts 300 ms: opacity advances linearly so the fade stays perceptible,
while position uses cubic ease-out. Exit begins after the existing two-second
idle hold, lasts 500 ms, and uses cubic ease-in for both channels. Thumb, track
background, and track border share the same opacity and translation so custom
styling remains visually coherent.

## State and Interruptions

The scrollbar state records opacity and position progress, its target
visibility, and the time at which the current transition began. Visibility
triggers are derived from the existing mode, scrolling, hover, and drag state.

When a trigger changes during an animation, the target reverses from the
current visual progress. It does not restart at an endpoint. In particular:

- Scrolling again during exit immediately reverses into the entrance.
- Hovering during exit reverses into the entrance.
- Dragging keeps the scrollbar fully visible until the drag ends.
- Leaving hover or ending a scroll/drag preserves the idle hold before exit.

Animation frames are requested only while progress is changing. The existing
idle timer wakes the view when the hold expires; no continuous frames are
requested during the hold.

## Painting and Interaction

Resolved styles and thumb geometry continue to use the current normal, hover,
and active style paths. Before painting, scrollbar quads are translated by the
axis-specific offset and their colors/backgrounds are multiplied by visibility
opacity. The stable, unshifted bounds continue to define mouse hitboxes.

Fully hidden scrollbars do not accept track clicks or thumb drags. `Hover` mode
continues observing pointer movement over the stable track bounds so it can
start the entrance. During entrance and exit, track clicks and thumb drags use
the same visibility rules as the current implementation; the animation does
not add an interaction region outside those stable bounds.

## Testing

Unit tests cover the pure visibility animation calculations:

- entrance endpoints, duration, monotonic opacity, and translation direction;
- idle hold followed by exit endpoints and duration;
- smooth reversal from an in-progress value;
- vertical and horizontal translation vectors;
- stationary, opaque behavior in `Always` mode.

Existing visual interaction tests continue to cover track clicks and thumb
dragging. The relevant crate test suite and formatting checks must pass after
the implementation.
