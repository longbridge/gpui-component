# Motion Core Design

**Status:** Approved direction; detailed design for review
**Date:** 2026-08-29
**Scope:** `gpui-base`, `gpui-component`, stories, documentation, and performance tests

## Purpose

Build a typed motion system that covers the CSS and Transitions.dev semantics
needed by a native desktop component library while preserving GPUI's ownership
and rendering model. The system must make polished motion easy, interruption
safe, accessible, and cheap enough to target a 120 Hz display.

This is not a CSS parser, a DOM animation cascade, or a complete Web Animations
API clone. CSS specifications are the behavioral reference for timing,
keyframes, easing, interruption, and presence. GPUI remains the renderer and
applications continue to own target state.

## User Outcomes

- Component authors can express transitions, springs, keyframes, enter/exit,
  stagger, and discrete swaps through one coherent API.
- Motion remains continuous when a target changes or an exit is reversed.
- Removed content can finish an exit transition before it is unmounted.
- Reduced motion reaches the stable UI immediately and does not leave a frame
  loop running.
- A settled animation has no timer, task, allocation, notification, or frame
  request cost.
- Standard components share timing, easing, distance, and spring tokens while
  `gpui-base` remains presentation-neutral.

## Current State

### What already works

`gpui-base::motion` provides:

- keyed target-value transitions with duration, positive delay, easing,
  interruption from the current sampled value, and reduced-motion handling;
- keyed physical springs with velocity-preserving retargeting, configurable
  damping and tolerance, and pointer-drag suspension;
- interpolation for the legacy `Lerp` types;
- animation-frame requests only while a transition or spring is active.

`gpui-base::animation` provides a CSS-correct cubic Bézier solver and a legacy
element effect builder. GPUI itself provides callback-based `Animation`, repeat,
and several easing helpers.

Styled components already animate selected behaviors, including checkbox and
switch values, slider affordances, accordion height, tabs, dropdown entrance,
dialog and sheet entrance, notifications, progress, skeletons, and spinners.
Toast, scrollbar, and dock behavior also contain specialized motion.

### Missing system behavior

The current pieces do not form one complete motion model:

1. There is no typed keyframe track with offsets and per-segment easing.
2. There is no shared presence lifecycle for enter, exit, and interrupted exit.
3. There is no common iteration, direction, phase, or completion model.
4. CSS `steps()` and piecewise `linear()` easing are absent.
5. Discrete value switching and stagger/sequence calculations are absent.
6. Negative delay and CSS-like transition reversal shortening are absent.
7. Composite values and paint-oriented transforms require callers to maintain
   multiple unrelated channels.
8. Component timings and curves are scattered constants rather than semantic
   theme tokens.
9. Several components use GPUI `Animation` directly and therefore do not share
   the same reduced-motion and lifecycle policy.
10. There is no motion-specific release benchmark or frame-budget regression
    suite.

## Reference Semantics

The design follows these useful parts of the web model:

- Web Animations separates a stateless timing model from the values an effect
  produces. Sampling is based on timeline time, not accumulated frame deltas.
- CSS Transitions starts from the value sampled at interruption and defines a
  reversing-shortening behavior for rapid reversal.
- CSS Transitions Level 2 introduces starting styles for newly-present content
  and opt-in discrete transitions.
- CSS Easing Level 2 defines linear, cubic Bézier, piecewise `linear()`, and
  `steps()` families.
- CSS Animations defines delay, duration, iteration count, direction, and
  per-keyframe easing.

The design intentionally omits browser-only machinery:

- CSS text parsing, selectors, cascade, and property discovery;
- DOM pseudo-elements and animation event dispatch;
- arbitrary animation composition stacks (`add` and `accumulate`);
- scroll/view timelines in the first implementation;
- persistent fill layers that override application target state indefinitely;
- implicit animation of every field in `StyleRefinement`.

Applications own final state. Motion supplies sampled values while active and
then disappears from the cost model.

## Architecture

```text
application state
      |
      v
gpui-component choreography + MotionTokens
      |
      v
gpui-base timing / tracks / presence / interpolation
      |
      v
GPUI keyed element state + request_animation_frame + paint/layout APIs
```

### `gpui-base`: behavior and math

Base owns:

- timing policies and deterministic sampling;
- easing functions;
- typed interpolation;
- keyed transition and spring state;
- keyframe tracks;
- presence state machines;
- stagger calculations;
- reusable geometry required for paint and layout transitions;
- reduced-motion resolution;
- active/settled status and lifecycle observations.

Base does not choose durations, distances, opacity values, component names,
colors, or choreography.

### `gpui-component`: visual policy

The styled layer owns:

- semantic motion tokens;
- component-specific enter, exit, selection, feedback, and loading motion;
- mapping theme tokens to Base policies;
- accessibility choices for reduced motion;
- stories demonstrating the supported transition vocabulary.

## Motion Core API

Names below are the intended public seams. Exact generic bounds may be refined
during TDD, but the ownership and behavior are requirements.

### Easing

Introduce a cheap cloneable `Easing` value instead of storing an `Rc<dyn Fn>`
for common cases:

```rust,ignore
pub enum Easing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
    Steps { count: NonZeroU32, position: StepPosition },
    LinearStops(Arc<[LinearStop]>),
    Custom(Rc<dyn Fn(f32) -> f32>),
}

impl Easing {
    pub fn sample(&self, progress: f32) -> f32;
}
```

CSS keyword curves use their specified control points, not similarly named
polynomial approximations. `Steps` supports jump-start, jump-end, jump-none,
and jump-both. `LinearStops` resolves omitted input positions at construction,
so frame sampling is a bounded binary search and interpolation with no
allocation.

Existing easing functions remain available for compatibility. New policies
accept `impl Into<Easing>` or expose an `ease(Easing)` builder.

### Timing sample

Separate timing math from keyed state:

```rust,ignore
pub struct Timing {
    pub delay: SignedDuration,
    pub duration: Duration,
    pub iterations: IterationCount,
    pub direction: PlaybackDirection,
    pub easing: Easing,
}

pub struct TimingSample {
    pub phase: MotionPhase,
    pub directed_progress: f32,
    pub iteration: u64,
    pub active: bool,
    pub finished: bool,
}
```

`Timing::sample(elapsed)` is deterministic and independent of frame count.
Boundary behavior is endpoint-exclusive during active iterations and yields
the intended endpoint when finishing. Infinite iteration is explicit and is
disabled under reduced motion unless a component supplies a non-motion static
fallback.

### Value transition

Keep the ergonomic `transition(...) -> T` API and add a status-returning form:

```rust,ignore
pub struct MotionValue<T> {
    pub value: T,
    pub status: MotionStatus,
}

pub fn transition_with_status<T>(...) -> MotionValue<T>;
```

`MotionStatus` distinguishes `Idle`, `Delayed`, `Running`, and `Finished`.
Target changes sample the current value before replacing the target. A direct
reversal uses the CSS reversing-adjusted start value and reversing shortening
factor so a nearly completed transition does not take a full duration to go
back. A change to a third target starts from the sampled value without applying
the reversal shortcut.

Negative delays begin partway through the active interval. A delay beyond the
end resolves immediately. Zero duration and reduced motion synchronize keyed
state and return the target without requesting a frame.

### Keyframe track

Provide immutable, validated tracks:

```rust,ignore
pub struct Keyframe<T> {
    pub offset: f32,
    pub value: T,
    pub easing: Easing,
}

pub struct Keyframes<T> { /* Arc<[Keyframe<T>]> */ }

impl<T: Interpolate> Keyframes<T> {
    pub fn try_new(frames: impl IntoIterator<Item = Keyframe<T>>)
        -> Result<Self, KeyframeError>;
    pub fn sample(&self, progress: f32) -> T;
}
```

Tracks require at least two frames, finite monotonic offsets in `[0, 1]`, and
explicit endpoints. Validation and storage allocation occur at construction,
never during sampling. Segment lookup uses binary search. The easing on a frame
applies to the segment beginning at that frame, matching CSS keyframe behavior.

A keyed `animate_keyframes` function combines `Timing` with a track and returns
`MotionValue<T>`. It supports finite/infinite iteration and all playback
directions. The settled state stops requesting frames.

### Discrete values

Provide a wrapper whose switch point is explicit:

```rust,ignore
pub struct Discrete<T> {
    pub from: T,
    pub to: T,
    pub switch_at: f32,
}
```

The default switch point is `0.5`, corresponding to CSS discrete transitions.
Components can use `0.0` or `1.0` for icon/content swaps where one side must be
fully hidden first. Discrete values do not pretend to be numerically
interpolatable.

### Composite motion values

Add interpolation for common GPUI geometry (`Size<Pixels>`, `Bounds<Pixels>`,
and color where supported) and a presentation-neutral transform value:

```rust,ignore
pub struct MotionTransform {
    pub translation: Point<Pixels>,
    pub scale: Point<f32>,
    pub rotation_radians: f32,
    pub opacity: f32,
}
```

The transform is a value, not a styling API. A rendering adapter applies only
the operations GPUI can execute correctly. Translation and opacity use native
paint/style paths. Scale and rotation require a custom element that transforms
paint and hit-test geometry consistently; they must not be advertised until
that invariant is proven. Unsupported transforms fail at the typed boundary
rather than silently degrading.

### Stagger and sequence

Stagger is pure timing arithmetic:

```rust,ignore
pub struct Stagger {
    pub interval: Duration,
    pub origin: StaggerOrigin,
}

impl Stagger {
    pub fn delay(&self, index: usize, count: usize) -> Duration;
}
```

Origins are first, last, center, or an explicit index. Reduced motion resolves
all delays to zero. No task, timer, vector, or child animation object is
created. A sequence is expressed by applying computed offsets to tracks rather
than by installing callbacks between animations.

## Presence

Presence solves a lifecycle problem, not merely opacity.

```rust,ignore
pub enum PresencePhase {
    Entering,
    Present,
    Exiting,
    Absent,
}

pub struct Presence { /* keyed retained state */ }
```

Given a requested `present: bool`, keyed presence returns a phase and progress:

- first mounted with `true`: enter from the configured starting value;
- stable `true`: present with no frame request;
- changed to `false`: remain mounted while exit runs;
- exit completes: report absent so the owner can remove content;
- changed back to `true` during exit: reverse from the sampled value without a
  visual jump;
- reduced motion: immediately report present or absent;
- changed element identity: start a new lifecycle intentionally.

Presence cannot keep an already-unmounted child alive by magic. The owning
component must retain the child or its render closure until Base reports
`Absent`. Compound overlay systems keep focus trapping, hit testing, and
dismissal semantics under their existing owner. During exit, an overlay no
longer accepts new interaction unless its component explicitly requires it.

Completion is observed as state returned during render, not an arbitrary
per-frame callback. A component that emits a semantic dismissal event does so
once when its lifecycle reaches `Absent`.

## Layout Motion

Layout-changing properties are inherently more expensive than paint-only
properties. The library supports them deliberately rather than pretending all
CSS properties have the same cost.

### Measured size transition

Accordion, collapsible, and content-resize effects use one behavior element
that:

1. measures natural content size in prepaint;
2. stores the measurement in keyed element state;
3. invalidates only when content, available width, or rem size changes;
4. animates the clipped outer size;
5. preserves content and hit-test masking;
6. requests a new frame only while measurement or motion is unsettled.

The existing accordion implementation is the starting point but the reusable
geometry belongs in Base.

### FLIP transition

FLIP is reserved for bounded collections, selection indicators, reordering,
and card resize where previous and next bounds can be measured cheaply. It
stores the prior rectangle, computes an inverse translation/scale, and animates
that transform to identity. It must not force full-list measurement or disable
virtualization.

Where GPUI cannot apply scale without incorrect paint or hit testing, the first
version uses translation-only FLIP plus an explicit clipped size transition.

## Motion Tokens

`gpui-component` adds semantic tokens instead of component-specific constants:

```rust,ignore
pub struct MotionTokens {
    pub duration_instant: Duration,
    pub duration_fast: Duration,
    pub duration_normal: Duration,
    pub duration_slow: Duration,
    pub easing_enter: Easing,
    pub easing_exit: Easing,
    pub easing_move: Easing,
    pub spring_control: Spring,
    pub spring_move: Spring,
    pub distance_short: Rems,
    pub distance_medium: Rems,
}
```

Defaults are initially derived from current successful component timings, then
tuned together in the motion story. Tokens describe intent rather than naming
Dialog, Toast, or Switch. Theme application projects relevant policy to
Base-owned deep behaviors, as scrollbar motion already does.

Reduced motion is not a second token theme. `cx.reduce_motion()` is the final
capability gate in Base. The styled layer may additionally replace a spatial
transition with a short opacity change only when the platform preference and
product policy permit it; the Base default is immediate settlement.

## Component Choreography

The initial migration covers existing animated components before adding more
decorative effects:

| Pattern | Components | Motion |
| --- | --- | --- |
| controlled value | Checkbox, Switch, Slider, Progress | spring/transition tokens |
| moving indicator | Tabs, TabBar | velocity-preserving spring |
| measured reveal | Accordion, Collapsible | shared size transition |
| anchored surface | Popover, Menu, Select, Combobox, Tooltip | origin-aware presence |
| modal surface | Dialog, AlertDialog, Sheet | backdrop + surface presence |
| transient stack | Notification/Toast | presence + layout spring |
| perpetual status | Spinner, Skeleton, indeterminate Progress | keyframes with static reduced-motion fallback |

Transitions.dev patterns then become stories built from the primitives:

- card resize: measured size/FLIP;
- number or text swap: presence + discrete swap + stagger;
- notification badge and success check: keyframes or spring tracks;
- menu, modal, panel, tooltip, accordion, and toast: presence choreography;
- tabs sliding: retained layout spring;
- skeleton reveal: finite cross-fade;
- error shake: finite translation keyframes;
- spinner-to-check: discrete swap plus keyframes;
- banner stacking: presence plus layout spring.

Blur-heavy filters, particle systems, 3D tilt, masks, and gooey effects are not
general component defaults. They may be opt-in stories after paint performance
and renderer support are demonstrated. This follows Transitions.dev's own
performance lesson: move effects out of repaint-heavy mask/filter paths when a
translated static layer is visually equivalent.

## 120 Hz Performance Contract

A 120 Hz display provides approximately 8.33 ms for the entire frame. Motion
Core does not claim the whole budget. Its CPU target is:

- timing and value sampling: at most 0.10 ms for 1,000 active scalar tracks on
  the benchmark machine;
- no heap allocation in a steady active-frame sample;
- no application task or entity notification per animation frame;
- no frame request after all sampled tracks settle;
- O(active tracks) work, with no scan of settled or absent animation IDs;
- one time read per render pass where batching is possible;
- layout animation used only where geometry changes are required;
- paint-only translation/opacity preferred for overlays and transient feedback.

The 0.10 ms microbenchmark is a library regression threshold, not proof that a
whole application renders at 120 FPS. End-to-end stories also record frame time
and dropped-frame tails in release builds. Passing means the motion layer leaves
sufficient budget for layout, text, painting, compositing, and the platform.

Benchmark results are recorded with hardware, OS, build profile, sample count,
median, p95, and worst observed frame. Debug measurements are diagnostic only.

## Testing

### Deterministic unit tests

- CSS reference samples for keyword cubic Bézier, `steps()`, and `linear()`;
- timing boundaries, negative delay, iteration count, and every direction;
- keyframe validation and per-segment easing;
- discrete switch points;
- transition interruption, direct reversal, and third-target retargeting;
- spring velocity preservation and settlement;
- presence enter, exit, re-entry, identity reset, and reduced motion;
- stagger origins and empty/single-item input;
- composite geometry interpolation;
- zero duration and reduced motion never request another frame.

Tests use GPUI's deterministic background executor clock where keyed window
state is involved. Pure timing and easing tests are ordinary Rust tests.

### Component tests

- exiting overlays remain mounted until completion but stop accepting input;
- focus restoration and dismissal events happen once;
- repeated open/close does not replay unrelated children;
- accordion measurement invalidates on width/rem/content changes;
- indefinite loaders settle to an accessible static representation under
  reduced motion;
- stable domain IDs isolate concurrent tracks.

### Performance tests

- release microbenchmarks for 1, 100, and 1,000 scalar tracks;
- keyframe sampling with 2, 8, and 32 frames;
- active versus settled frame-request counts;
- allocation counting around the steady sampling loop;
- story-level 120 Hz load cases for overlay presence, a toast stack, tabs, and
  measured reveal.

## Compatibility and Migration

- Existing `transition` and `spring` call sites continue to compile.
- `animation::EffectTransition` remains deprecated but is not removed in this
  change.
- New easing values gain conversions from existing function-based policies
  where practical; custom closures remain supported.
- Components migrate one behavior family at a time, with visual and lifecycle
  tests before deleting old constants.
- Public component defaults may change only where the existing behavior is
  inconsistent, fails reduced motion, or causes lifecycle defects. Timing
  changes are documented.

## Documentation Deliverables

Motion is a first-class documented capability, not an API discoverable only by
reading source and stories. Completion includes a dedicated English and Chinese
documentation set linked from the website navigation:

- `website/docs/motion.md` and `website/zh-CN/docs/motion.md`: conceptual guide
  covering ownership, timing, transition versus spring, keyframes, presence,
  stagger, layout motion, interruption, stable identity, and reduced motion;
- a public API capability matrix showing supported values and semantics,
  deliberate CSS/Web Animations omissions, and the appropriate lower-level
  escape hatch;
- copyable examples for controlled values, enter/exit, interrupted presence,
  keyframes, stagger, measured reveal, and perpetual loading with a static
  reduced-motion fallback;
- a performance chapter explaining the 8.33 ms whole-frame budget, paint-only
  versus layout-changing properties, allocation/frame-request rules, benchmark
  commands, and how to interpret results without treating a microbenchmark as
  end-to-end proof;
- a migration section mapping legacy `Animation`, `EffectTransition`, and
  component-local constants to Motion Core;
- rustdoc on every public type and builder, including lifecycle, identity,
  reduced-motion, interruption, and performance behavior;
- a motion story that visually demonstrates the documented primitives and
  references the same semantic token names.

The documentation must distinguish guarantees from recommendations. It must
not claim browser CSS syntax compatibility, compositor promotion, or universal
120 FPS. It reports what the library controls and provides reproducible release
measurements for the rest.

## Delivery Order

1. Easing value, pure timing sampler, keyframes, and benchmarks.
2. Transition status, negative delay, reversal semantics, and composite values.
3. Presence, discrete values, and stagger.
4. Motion tokens and migration of controlled-value/indicator components.
5. Overlay presence and focus-safe exit migration.
6. Shared measured-size transition and layout motion.
7. Dedicated English/Chinese motion documentation and migration guide.
8. Transitions.dev motion story and end-to-end performance audit.

Each stage leaves independently usable, tested software. Performance tests are
introduced with the first core stage rather than appended after migration.

## Completion Criteria

The work is complete only when:

1. every API described as part of Motion Core is implemented and documented;
2. all existing animated components use the shared core or have a documented
   reason to retain a specialized native path;
3. every listed Transitions.dev pattern is either demonstrated or explicitly
   classified as an opt-in renderer experiment;
4. reduced motion is verified across finite, presence, and indefinite motion;
5. deterministic lifecycle and interruption tests pass;
6. release benchmarks satisfy the Motion Core sampling and allocation budget;
7. story-level measurements record whether representative scenes sustain the
   8.33 ms frame budget, without presenting a microbenchmark as whole-frame
   proof;
8. settled interfaces stop requesting animation frames;
9. the dedicated English/Chinese motion documentation, API capability matrix,
   migration guide, rustdoc, and linked motion story are complete and agree
   with the implemented public API.

## Sources

- CSS Transitions Level 2: <https://www.w3.org/TR/css-transitions-2/>
- Web Animations Level 1: <https://www.w3.org/TR/web-animations-1/>
- CSS Animations Level 2: <https://www.w3.org/TR/css-animations-2/>
- CSS Easing Functions Level 2: <https://www.w3.org/TR/css-easing-2/>
- CSS View Transitions Level 2: <https://www.w3.org/TR/css-view-transitions-2/>
- Transitions.dev: <https://transitions.dev/>
