# Visual Hierarchy, Colors, and Tokens

## Hierarchy & Emphasis Budget

Prefer a small number of clear levels:
- **Window or page title** identifies the current object or workspace;
- **Section title** separates meaningful regions;
- **Body text** carries the work;
- **Muted text** provides secondary metadata and help;
- **Labels** identify controls and values.

Use size, weight, spacing, and separators before adding color or containers. Avoid nesting cards inside cards: most desktop regions need only a background, a hairline boundary, and intentional spacing.

Treat emphasis as a limited budget. A local surface needs one clear focal point. If everything is colored, badged, bold, boxed, or promoted to an alert, nothing reads as important.

---

## Colors and Themes

Read colors from `cx.theme()` and use them by semantic role:
- `background` and `foreground` for the main surface and text;
- `card`, `popover`, `sidebar`, and their foreground tokens for named surfaces;
- `muted` and `muted_foreground` for supporting information;
- `primary` for the principal action or selection emphasis;
- `danger`, `warning`, `success`, and `info` only for their meanings;
- `border`, `input`, and focus-ring tokens for structure and interaction.

Rules:
- Do not use a semantic status color as decoration. Do not encode meaning by color alone.
- Application UI should not contain raw hex, `rgb`/`rgba`, or `hsla` colors.
- Use Badge for short states, counts, or classifications—not for every label or section title. Keep most badges neutral.

---

## Density Tiers

Medium is the ecosystem default:
- **comfortable / large:** onboarding, sparse forms, prominent decisions;
- **standard / medium:** most application chrome and workflows;
- **compact / small:** toolbars, menus, tables, and repeated professional data;
- **extra compact / xsmall:** exceptional high-density utilities.

---

## Surfaces, Elevation, and Typography

- Use elevation (shadows) to explain stacking (popovers, menus, dialogs), not importance. The base window surface is flat.
- Use the platform UI font for interface text and monospace only for code, identifiers, shortcuts, and tabular data.
- Use one icon family. Icons supplement labels; icon-only buttons require a tooltip and accessible name.
