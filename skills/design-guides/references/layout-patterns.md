# Desktop Layout Patterns & Window Shells

## Stable Window Shells

Most applications should use one of these standard shells:
- **single workspace:** toolbar or title bar above one primary view;
- **sidebar workspace:** persistent navigation beside a changing detail view;
- **master–detail:** resizable collection and detail panes;
- **document workspace:** tabs or a dock area for multiple long-lived objects;
- **utility window:** one focused task with a short, fixed action path.

Keep global navigation stable while content changes. Give the primary work area the remaining space with `flex_1()` and `min_w_0()` / `min_h_0()` where overflowing children must shrink.

---

## Responsive Desktop Windows

When a window narrows:
1. Preserve the primary task;
2. Allow resizable regions to reach a documented minimum;
3. Collapse secondary labels or inspectors;
4. Move low-frequency actions into a menu;
5. Scroll only the region whose content actually overflows (never make the whole window scroll when only a list should scroll).

---

## Forms and Settings

- Use a visible label for each field and place help or validation next to the field it describes.
- Use the appropriate control:
  - `Checkbox` for independent choices;
  - `RadioGroup` for a small visible set;
  - `Select` for a longer set;
  - `Switch` for a setting that takes effect immediately.
- Disable submission while an operation is in flight, retain user input, and show the result near the action.
- Reserve dialogs for short, focused decisions; use full pages or sheets for complex multi-field workflows.
