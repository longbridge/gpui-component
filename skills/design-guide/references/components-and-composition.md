# Components and Composition

Components are building material rather than a sealed design system. GPUI Component supplies coherent defaults, while the application owns composition and product semantics.

## Principles of Composition

- **Use component variants by meaning**:
  - `primary` is reserved for the explicit default commit in a decision area (invoked by Enter);
  - `default` / `outline` for ordinary visible actions (e.g. `Add` in a toolbar);
  - `danger` for destructive commitments;
  - `ghost` for quiet toolbar actions.
- **Prefer explicit compound parts and render callbacks** over styling arbitrary descendants.
- **Use the standard component for its semantic role**: Menus, dropdowns, popovers, selects, and command palettes each own different selection, keyboard, focus, and dismissal contracts. Do not rebuild custom popups from `div`s.
- **Preserve component family geometry**: Menu rows share height, padding, icon slots, separators, and radius.
- **Move reusable unstyled behavior to `gpui-base`**; keep styled presentation in `gpui-component` or the application.
