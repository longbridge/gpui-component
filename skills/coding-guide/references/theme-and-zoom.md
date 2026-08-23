# Theme, Styling, and Application Zoom

Read semantic values from the active theme and apply layout with GPUI's `Styled` methods:

```rust
div()
    .bg(cx.theme().background)
    .text_color(cx.theme().foreground)
    .border_1()
    .border_color(cx.theme().border)
    .rounded(cx.theme().radius)
```

## Styling Rules

- **Do not hard-code product colors, corner radii, spacing, or control geometry.**
- Application code **must not introduce raw hex, `rgb`/`rgba`, or `hsla`**; read a semantic color from `cx.theme()` or add the missing role to the product theme.
- Application layout **should use GPUI's rem-based scale helpers (`p_2()`, `gap_3()`, `w_64()`, `text_sm()`) instead of direct `px(...)` values.**
- Use semantic tokens for meaning, not palette position.
- Use GPUI `hover`, `active`, `focus`, and `focus_visible` modifiers for runtime interaction states.
- Use a component's semantic state styles for checked, selected, pressed, or disabled appearance.
- Guard hover/active refinements when a disabled control must not react.
- Keep Badge and Alert variants semantic and scarce. Ordinary metadata stays neutral.
- If code mutates the global GPUI Component theme directly, call `Theme::sync_base(cx)` afterward so Base-owned scrollbars and resize handles receive the new projection.
- An outward focus ring needs physical room. An ancestor with `overflow_hidden()` clips it. Retain room or use the theme's focus-ring policy.

---

## Base Font is the Application Zoom Control

`Root::render` calls `window.set_rem_size(cx.theme().font_size)`. Therefore the theme's base font is not only body typography; it is the reference length for the application's rem-based design scale.

Change zoom by updating the base font and refreshing the window:

```rust
Theme::global_mut(cx).font_size = px(18.);
Theme::sync_base(cx);
window.refresh();
```

- The base font itself is a pixel value because it anchors the scale. Descendant application UI should normally use relative helpers—`text_sm()`, `gap_2()`, `px_3()`, `h_8()`, `size_4()`—so type, whitespace, controls, and icons respond together.
- Treat every direct `px(...)` and raw color constructor in application UI as a review finding. Accept it only for a documented physical/platform boundary, measured runtime geometry, raster/data color, or the theme/token definition itself.
- **Cache invalidation with rem zoom**: Anything cached from resolved layout must include `window.rem_size()` in its invalidation key (wrapped row heights, text shaping/layout, virtual-list measurement, popup geometry, canvas metrics).
- Do not confuse application zoom with Dock panel zoom. Dock zoom makes one tab group or tile fill the DockArea while keeping container chrome; it must not modify window rem size.
