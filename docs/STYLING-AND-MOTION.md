# Styling and Motion

## Scope

This document defines how GPUI runtime interaction styles, base semantic-state
styles, application styles, and motion compose. It describes the current public
contract rather than a future milestone plan.

## Ownership

```text
GPUI
  detects hover, active, focus, and focus-visible state

gpui-base
  defines semantic component states
  resolves semantic-state style precedence
  provides generic value-transition lifecycle

application or gpui-component
  owns all target styles
  owns variants and visual slots
  chooses animated properties and timing
```

The phrase “base owns state” is intentionally avoided. Applications commonly
own controlled values such as `checked`, `selected`, and `open`. Base defines
how those values affect behavior and how their optional semantic styles are
resolved.

## GPUI Interaction Styles

Use GPUI's native modifiers for runtime pseudo-states:

```rust,ignore
element
    .hover(|style| style.bg(hover))
    .active(|style| style.bg(active))
    .focus(|style| style.border_color(focus))
    .focus_visible(|style| style.border_color(ring))
```

GPUI resolves these states at runtime in its own fixed order. Base does not
provide a second hover, active, or focus selector API.

A GPUI pseudo-state has one owner. Repeated registration of modifiers such as
`hover` may assert in debug builds, and base cannot read or merge GPUI's private
interaction refinements from another crate.

## Semantic-State Styles

Semantic states describe component values rather than pointer conditions:

- checked and indeterminate;
- pressed;
- selected;
- focused when it is a component value contract;
- disabled.

Controls expose only states that they can actually enter. For example, a Button
does not expose a checked style and a Slider does not expose an open style.

```rust,ignore
Checkbox::new("terms")
    .checked(checked)
    .disabled(disabled)
    .border_1()
    .styles(|styles| {
        styles
            .checked(|style| style.bg(primary))
            .indeterminate(|style| style.bg(primary))
            .disabled(|style| style.opacity(0.5))
    })
```

`StateStyle` implements GPUI's `Styled` interface and `FluentBuilder`, so state
closures can use normal style methods and helpers such as `when`, `when_some`,
and `when_none`. It is not a separate styling language.

## Style Precedence

Every base control resolves static and semantic styles in this order:

```text
instance style
→ active value states in the component's documented order
→ disabled
→ GPUI runtime interaction refinements
```

Later layers override only the fields they set. Unrelated fields from earlier
layers remain intact.

For a Checkbox, a typical value-state order is:

```text
instance → checked → indeterminate → disabled
```

Normalized checkbox state makes checked and indeterminate mutually exclusive,
but the fixed ordering still keeps resolution deterministic.

The order in which closures are written inside `.styles(...)` does not change
precedence. The component defines the state order and routes it through the
shared resolver.

### Preserving an application style in an active state

If a compatibility component requires a caller-provided style to win over one
semantic state, replay that refinement inside the state closure:

```rust,ignore
Button::new("save")
    .bg(brand)
    .styles(|styles| {
        styles.disabled(|style| style.opacity(0.5).bg(brand))
    })
```

Base cannot infer which fields in one `StyleRefinement` came from component
defaults and which came from a final caller override.

### Disabled interaction appearance

Base controls suppress activation while disabled. GPUI does not expose native
hover and active refinements for base to remove, and those refinements run after
semantic styles. Guard interaction modifiers at the call site when disabled
controls must not react visually:

```rust,ignore
.when(!disabled, |element| {
    element
        .hover(|style| style.bg(hover))
        .active(|style| style.bg(active))
})
```

## Root and Part Styling

Semantic root styles do not automatically traverse into children. Compound
parts are explicit application-owned styling boundaries:

```text
Checkbox / CheckboxIndicator
Switch / SwitchTrack / SwitchThumb
Slider / SliderTrack / SliderIndicator / SliderThumb
```

Each part exposes the semantic state needed to style itself. Applications
should construct and style the parts directly instead of expecting a root style
to mutate arbitrary descendants.

Keep state-independent geometry in the part's ordinary builder chain and put
state-dependent color, border, fill, or opacity in its semantic style context.

## Motion Ownership

Ordinary semantic controls do not install default fade, slide, spring, or size
animations. Product motion is presentation and therefore belongs to the
application or the styled component layer.

The base crate provides a generic target-value transition:

```rust,ignore
let opacity = gpui_base::transition(
    ("dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    gpui_base::Transition::new(Duration::from_millis(160)),
    window,
    cx,
);
```

The transition owns lifecycle mechanics only:

- keyed retained state;
- duration and delay;
- easing;
- animation-frame requests;
- smooth reversal from the currently sampled value;
- reduced-motion handling.

The caller chooses what the value means and applies it to opacity, color,
geometry, or another interpolatable property.

Deep behavior modules may own configurable motion when it is required to keep
their internal layout lifecycle coherent. `ToastStack`, for example, combines
measurement, overlap, expansion, and collapse through `ToastMotion`. This does
not give base ownership of toast colors, typography, borders, or content.

`Scrollbar` follows the same rule through `ScrollbarMotion`. A scrollbar is a
custom element that paints its own track and thumb, so no caller can apply an
opacity or offset to it from outside. Base therefore plays the transition, but
owns none of its timing: `ScrollbarMotion::default()` has zero enter, exit, and
expand durations, so an unstyled scrollbar appears and disappears without
motion. The styled layer projects product timing and the entrance choreography
through `ScrollbarTheme::motion`. A zero duration always means "adopt the
target now", which is also how reduced motion and always-visible scrollbars
reach the same code path.

## Transition Identity

A transition ID identifies one independently animated value. Use a stable
element-like ID and a named channel when one component animates multiple values:

```rust,ignore
("checkbox", "indicator-opacity")
("checkbox", "indicator-scale")
```

Do not reuse one ID for different value types or unrelated component instances.
State is keyed within the current GPUI element-state scope.

## Target Changes and Reversal

When a target changes during an active transition, the next transition begins
from the value sampled at that instant. It does not restart from the previous
endpoint. This prevents discontinuities during rapid toggles.

On the first render, the target is adopted immediately. When reduced motion is
enabled or duration is zero, the target is returned immediately and retained
transition state is synchronized with it.

## Supported Values

`transition` accepts values implementing `Interpolate`, `Clone`, and
`PartialEq`. Types implementing the legacy `animation::Lerp` trait receive an
`Interpolate` implementation automatically.

Applications may implement `Interpolate` for their own value types when the
interpolation is meaningful and deterministic.

## Legacy Element Animation

`motion::Transition` is the preferred API for application-owned target values.
It is distinct from `animation::Transition`, the legacy element-animation API
that applies concrete fade, slide, or size effects.

New component code should prefer value transitions because the presentation
owner explicitly selects the animated property. Existing legacy animation code
may continue to use its module-qualified API.

## Design Invariants

1. Static semantic styling must not require retained animation state.
2. Base controls must remain usable without motion.
3. Base must not choose visual target values for application components.
4. GPUI native pseudo-states remain the only hover, active, and focus styling
   mechanism.
5. Semantic state precedence is fixed by the component, not builder call order.
6. Disabled is the last semantic layer.
7. Part styling is explicit and typed; base does not traverse arbitrary child
   trees to apply styles.
8. Reduced-motion preferences are honored by generic transitions.
