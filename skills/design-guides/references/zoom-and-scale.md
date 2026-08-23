# Zoom, Base Font, and Rem Scaling

A well-designed `rem` system preserves hierarchy while the interface zooms. Zoom is successful when the relationship between title and body, control and icon, inner and outer spacing, primary and secondary regions still feels the same at every scale.

## Relative Design Scale

- The theme's base `font_size` becomes the window's `rem` through `Root`.
- GPUI scale helpers (`text_sm()`, `gap_2()`, `p_4()`, `h_8()`, `size_4()`) resolve against it.
- Typography, spacing, controls, and icons share one zoom axis.

## Rules for Application Layout

- **Do not call `px(...)` directly in application layout.** Use GPUI's rem-based scale helpers or semantic component sizes.
- **Fixed pixels are reserved exclusively for physical/raster boundaries**: 1-device-pixel hairlines, platform window insets, bitmap dimensions, and minimum hit-test bounds.
- **Design in ratios**:
  - Type steps maintain hierarchy around base body size;
  - Spacing steps maintain grouping relationships;
  - Control frames and hit targets scale with labels;
  - Pane minima and comfortable widths account for scaled content.
- Do not implement zoom by changing text size alone (causes clipped fixed-height boxes).
- Test interface zoom across multiple base-font values (e.g. 14px, 16px, 18px).
