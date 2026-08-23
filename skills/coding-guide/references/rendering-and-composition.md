# Rendering, Composition, and Presentation Boundary

## Declarative Rendering

Keep `render` declarative: read current state, derive presentation values, and compose elements. Move domain operations, parsing, and non-trivial mutation to named methods or services.

```rust
impl Render for ProjectView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(self.render_content(cx))
    }
}
```

- Extract a **render helper** when it names a meaningful region and reduces the amount of state a reader must hold at once.
- Extract a **new component** when the region has its own reusable contract or retained lifecycle—not merely because a builder chain is long.
- Use GPUI Component's fluent traits consistently (`Sizable`, `Disableable`, `Selectable`, and component-specific builders).
- Prefer `.when(...)` and `.when_some(...)` for small conditional refinements; use ordinary Rust control flow when branches represent substantially different interfaces.

## Compose from Standard Semantic Components

Compose from the standard semantic component before building a custom surface. Do not reproduce a menu, select, dropdown, or command palette from generic `div`s merely to match one screenshot. Reusing the component preserves its item geometry, focus transfer, keyboard navigation, selection, disabled state, dismissal, and accessibility contract.

Render callbacks supplied by application code should be side-effect-free. A list item renderer, menu builder, or dock panel renderer may run whenever its owner needs to measure or redraw. It must not perform a business operation, append data, or register an unbounded subscription.

---

## Behavior and Presentation Boundary

The durable Base rule is:

> **Base owns reusable behavior and the geometry required to implement it. The presentation layer owns the product's visual language.**

- “Headless” does not mean “one empty `div`.” Popup collision, keyboard navigation, editing, virtualization, resize arithmetic, focus trapping, and dock reconciliation require internal structure and state.
- Conversely, Base must not choose brand colors, typography, density, final icons, component variants, or application composition.
- Expose presentation through `Styled`, typed semantic-state styles, explicit parts, child slots, and item renderers. Do not inspect arbitrary descendants to discover titles, descriptions, or close buttons—make semantic parts explicit.
