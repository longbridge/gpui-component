# Input Focus Ring Design

## Goal

Give UI input controls a two-layer focused appearance matching the pattern used by shadcn and reui: a solid focused border plus a wider translucent outer ring.

## Scope

Apply the new appearance to `Input` and input components that intentionally reuse its renderer, including `Textarea` and `NumberInput`. Keep `Editor` unchanged even though it currently renders through `Input::from_base`.

Do not change focus styling for unrelated controls such as buttons, checkboxes, radios, or OTP slots.

## Visual behavior

When an eligible, enabled input is focused and its appearance, border, and focus border are enabled:

- The existing one-pixel border uses the theme's solid `ring` color.
- A separate two-pixel outer ring uses the same `ring` color at 20% opacity.
- The outer ring follows the input's resolved corner radius and sits outside its border without affecting layout.

Unfocused and disabled inputs retain their existing appearance. `appearance(false)`, `bordered(false)`, and `focus_bordered(false)` suppress both focused layers.

## Implementation boundary

Keep the outer-ring drawing in the existing `FocusableExt::focus_ring` helper. Add an internal renderer option to `Input` so wrappers can opt out of the outer ring. `Editor` sets that option off; normal `Input`, `Textarea`, and the `Input` used by `NumberInput` keep it on.

The focused solid border remains in the current focused style path. The outer ring is applied after the component's final style refinement so its geometry follows the resolved border widths and radii.

## Verification

- Add focused-style coverage proving eligible inputs contain both the solid border and outer ring.
- Add coverage proving `Editor` does not receive the outer ring.
- Preserve the existing behavior of the appearance, bordered, focus-bordered, and disabled switches.
- Run formatting and the relevant UI/input tests.
