---
title: Motion
description: CSS-aligned timing, transitions, springs, keyframes, presence, reveal, and performance guidance for GPUI applications.
---

# Motion

GPUI Component splits motion into two layers:

- `gpui-base` owns deterministic sampling and lifecycle: easing, timing, keyed state, interruption, reversal, frame requests, reduced motion, presence, and measured reveal.
- `gpui-component` owns visual policy: semantic durations, curves, springs, distances, and which component property moves.

This is intentionally not a CSS parser. It provides the useful animation semantics of CSS Transitions, CSS Animations, and Web Animations as typed Rust APIs that fit GPUI's retained element state.

## Capability map

| Capability | API | Notes |
| --- | --- | --- |
| Target transition | `transition`, `transition_with_status` | Stable identity, delay, easing, interruption, direct-reversal shortening |
| Physical motion | `spring` | Preserves velocity when retargeted; frame-rate independent |
| CSS easing | `Easing` | Keywords, cubic Bézier, `steps()`, and piecewise `linear()` stops |
| CSS timing | `Timing` | Signed delay, finite/infinite iterations, normal/reverse/alternate directions |
| Keyframes | `Keyframes`, `animate_keyframes` | Validated offsets and per-segment easing |
| Discrete values | `Discrete` | Switches non-interpolated state at an explicit progress point |
| Sequencing | `Stagger` | Allocation-free delay calculation from first, last, or center |
| Mount lifecycle | `Presence` | Enter, present, exit, absent; content remains mounted through exit |
| Measured expansion | `MotionReveal` | Clips a naturally measured child while revealing its height |
| Common composites | `MotionTransform` | Translation, scale, rotation, and opacity interpolation |
| Product policy | `MotionTokens` | Theme-owned durations, curves, springs, and distances |

## Target transitions

Give every independently animated value a stable ID. A named child channel is useful when one component animates multiple values.

```rust,ignore
use std::time::Duration;
use gpui_base::{Easing, Transition, transition};

let opacity = transition(
    ("save-dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut),
    window,
    cx,
);
```

Retargeting begins from the value sampled at that instant. A direct reversal also shortens the return duration in the same spirit as CSS Transitions, so a control reversed after 20% travel does not spend a full duration covering the remaining 20%.

Use `transition_with_status` when rendering depends on `Idle`, `Delayed`, `Running`, or `Finished`.

Positive delay waits before starting. A negative `SignedDuration` starts inside the active interval:

```rust,ignore
let policy = Transition::new(Duration::from_millis(200))
    .delay(SignedDuration::negative(Duration::from_millis(50)));
```

## Easing

`Easing` supports the CSS keyword curves (`Linear`, `Ease`, `EaseIn`, `EaseOut`, `EaseInOut`), arbitrary cubic Bézier curves, all CSS step positions, and normalized piecewise-linear stops.

```rust,ignore
let enter = Easing::cubic_bezier(0.16, 1.0, 0.3, 1.0);
let frames = Easing::steps(8, StepPosition::JumpEnd)?;
let emphasized = Easing::linear_stops([
    LinearStop::at(0.0, 0.0),
    LinearStop::new(0.15),
    LinearStop::new(0.8),
    LinearStop::at(1.0, 1.0),
])?;
```

Invalid step counts, stop positions, and Bézier x coordinates return typed errors rather than producing unstable samples.

## Keyframes and timing

`Timing` samples from absolute elapsed time, not accumulated frame deltas. Dropped or irregular frames therefore do not change the result.

```rust,ignore
let track = Keyframes::try_new([
    Keyframe::new(0.0, 0.0_f32).ease(Easing::EaseOut),
    Keyframe::new(0.6, 1.08).ease(Easing::EaseInOut),
    Keyframe::new(1.0, 1.0),
])?;

let value = animate_keyframes(
    "success-pop",
    &track,
    Timing::new(Duration::from_millis(280))
        .iterations(IterationCount::Finite(2))
        .direction(PlaybackDirection::Alternate),
    window,
    cx,
);
```

Offsets must start at `0`, end at `1`, and be monotonic. Sampling uses binary search, so larger tracks do not scan every keyframe.

For a non-interpolated property, use `Discrete::new(from, to).switch_at(progress)`.

## Springs

Use a spring for a value whose target may move while it is already moving: a selected-tab indicator, a dragged panel settling into place, or a rapidly toggled control.

```rust,ignore
let x = spring(
    ("tabs", "indicator-x"),
    selected_x,
    cx.theme().motion_tokens().spring_move,
    window,
    cx,
);
```

A transition preserves position across retargeting; a spring preserves both position and velocity. Do not spring a pointer-controlled value during the drag itself. Use `with_travel(false)` while dragging, then re-enable travel on release.

## Presence and exit motion

Conditional rendering normally removes a child immediately, which makes exit animation impossible. `Presence` separates logical presence from physical mounting:

```rust,ignore
let presence = Presence::new("command-palette", open)
    .transition(Transition::new(tokens.duration_normal).easing(tokens.exit))
    .sample(window, cx);

if presence.should_render() {
    // Apply presence.progress to opacity/translation and render the child.
}
```

The phases are `Entering`, `Present`, `Exiting`, and `Absent`. Reopening during exit reverses from the current sample. Under reduced motion, presence resolves immediately and schedules no extra frame.

## Measured reveal

`MotionReveal` keeps a child mounted at its natural size, measures it, and clips the visible height by normalized progress. `Collapsible::motion_id(...)` is the convenient styled API:

```rust,ignore
Collapsible::new()
    .motion_id("advanced-options")
    .open(show_advanced)
    .content(options)
```

Use a stable ID. Without `motion_id`, `Collapsible` preserves its original immediate mount/unmount behavior.

## Stagger

`Stagger` calculates one delay without allocating a schedule:

```rust,ignore
let stagger = Stagger::new(Duration::from_millis(24), StaggerOrigin::Center);
let delay = stagger.delay(index, item_count);
```

Origins can be first, last, or center. Stagger changes timing only; list identity and rendering remain the application's responsibility.

## Theme motion tokens

Use `cx.theme().motion_tokens()` in styled components instead of local timing constants. The token set includes:

- `duration_instant`, `duration_fast`, `duration_normal`, `duration_slow`;
- `enter`, `exit`, and `move` easing;
- `spring_control` and `spring_move`;
- `distance_short` and `distance_medium`.

The base crate does not select these product values. A custom design system can use the base motion engine with its own tokens.

## Reduced motion

Keyed transitions, springs, keyframes, presence, and reveal-compatible controls honor GPUI's reduced-motion preference. Finite motion snaps to the target, retained state is synchronized, and no animation frame remains pending. Infinite indicators use GPUI's native reduced-motion handling.

Motion must never be the only way a state or relationship is communicated.

## Performance

The steady sampling path is allocation-free. A release benchmark samples 1,000 values per batch and fails if scalar timing/easing exceeds `0.10 ms` median on the reference machine. The implementation requests animation frames only while delayed or active, uses absolute elapsed time, and binary-searches keyframes.

For a 120 Hz display, one frame is about `8.33 ms`; the motion sampler is designed to consume a small fraction of that budget. Application paint, layout, text shaping, and content still share the rest, so prefer opacity and paint transforms over large layout animations when either communicates the change.

Run the benchmark with:

```bash
cargo bench -p gpui-base --bench motion
```

## Choosing an API

- One target, known duration: `transition`.
- Frequently changing target or spatial continuity: `spring`.
- Authored multi-stage sequence: `Keyframes` plus `Timing`.
- Delayed list choreography: `Stagger` combined with a transition or keyframe timing.
- Exit before unmount: `Presence`.
- Expand/collapse measured content: `MotionReveal` or `Collapsible::motion_id`.
- Element-local infinite spinner/skeleton: GPUI native animation remains appropriate.
