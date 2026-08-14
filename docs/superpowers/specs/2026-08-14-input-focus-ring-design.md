# Input Focus Ring Design

## Goal

Give UI input controls a two-layer focused appearance matching the pattern used by shadcn and reui: a solid focused border plus a wider translucent outer ring.

## Scope

Apply the new appearance to form controls with input-like frames: `Input`, `Textarea`, `NumberInput`, `Select`, `Combobox`, `DatePicker`, and the active `OtpInput` slot. Keep `Editor` unchanged even though it currently renders through `Input::from_base`.

Do not change focus styling for unrelated controls such as buttons, checkboxes, or radios.

## Visual behavior

When an eligible, enabled input is focused and its appearance, border, and focus border are enabled:

- The existing one-pixel border uses the theme's solid `ring` color.
- A separate four-pixel outer ring uses the same `ring` color at 50% opacity. GPUI centers borders on their path, so the ring is offset by two pixels to keep its inner edge flush with the focused border.
- The outer ring follows the input's resolved corner radius and sits outside its border without affecting layout.

Unfocused and disabled inputs retain their existing appearance. `appearance(false)`, `bordered(false)`, and `focus_bordered(false)` suppress both focused layers.

The default theme follows shadcn's neutral ring contrast: `neutral-400` in light mode and `neutral-500` in dark mode. Keeping this softness in the semantic token lets the component use shadcn's opacity algorithm without local color overrides.

## Implementation boundary

Keep all outer-ring drawing in the UI crate. `gpui-base::FocusableExt` is a state-only component API: `.focus_ring(bool)` sets whether the UI layer may draw the ring and `.is_focus_ring_enabled()` reads that setting. Base must not read theme values, calculate geometry, or create visual ring elements. Width, opacity, spacing, color, and geometry remain private to the UI design system.

`Editor` continues to opt out through `focus_bordered(false)`. `NumberInput` applies the border and ring to a wrapper around its complete spinbutton rather than its nested appearance-free Input. Select, Combobox, and DatePicker keep clipping on their inner content rows but not on the outer frame that owns the ring. `OtpInput` applies both focused layers only to its active slot.

Form field content wrappers must allow horizontal overflow so a control's external ring remains visible.

The focused solid border remains in the current focused style path. The outer ring is applied after the component's final style refinement so its geometry follows the resolved border widths and radii.

## Verification

- Add focused-style coverage proving eligible inputs contain both the solid border and outer ring.
- Add coverage proving `Editor` does not receive the outer ring.
- Preserve the existing behavior of the appearance, bordered, focus-bordered, and disabled switches.
- Run formatting and the relevant UI/input tests.
