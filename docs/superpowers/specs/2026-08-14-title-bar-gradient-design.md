# TitleBar Default Gradient Design

## Goal

Improve the default `TitleBar` background with a subtle vertical gradient equivalent to:

```css
linear-gradient(
  to bottom,
  color-mix(in srgb, var(--titlebar) 55%, var(--background)),
  var(--titlebar)
)
```

The gradient applies to every `TitleBar` by default while preserving the existing ability for callers to override its background through `Styled` methods such as `.bg(...)`.

## Implementation

In `TitleBar::render`, replace the solid default background with GPUI's two-stop `linear_gradient` background.

- The top stop mixes 55% of `cx.theme().tokens.title_bar` with 45% of the theme background.
- The bottom stop is `cx.theme().tokens.title_bar`.
- The gradient runs from top to bottom.
- `refine_style(&self.style)` remains after the default background assignment, so caller-provided styles retain precedence.

The implementation will use existing GPUI color and background APIs. It will not add theme fields, alter the theme schema, or change the public `TitleBar` API.

## Compatibility

The change affects the default rendering on all supported platforms. Existing callers that set a custom background continue to render their chosen background. Theme authors continue to provide the same `title_bar` and `background` colors without migration.

## Verification

- Add or update a focused test to verify the default background is the expected gradient.
- Verify a caller-provided `.bg(...)` style still overrides the default.
- Run formatting and the relevant `gpui-component` UI tests.
