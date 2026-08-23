# Testing Strategy, Performance, and Pitfalls

## Testing Strategy

Test at the lowest layer that can prove the behavior:
1. **Pure tests** for state transitions, geometry, parsing, and ordering;
2. **GPUI context tests** for entities, events, and subscriptions;
3. **`VisualTestContext` interaction tests** for focus, keyboard, pointer, layout, and rendered state;
4. **Example or application smoke tests** for complete workflows.

For an interactive component, cover the semantic contract:
- Pointer and keyboard activation;
- Controlled value changes;
- Disabled behavior;
- Focus movement;
- Event count/order;
- Stable identity;
- Empty or failure states.

---

## Performance Rules

- **Do not mutate state or notify unconditionally in `render`.**
- Avoid rebuilding entities, subscriptions, focus handles, and expensive data structures per frame.
- Notify the narrowest owning entity after a coherent state change.
- Virtualize long collections and render only the visible range.
- Avoid cloning large strings or collections solely to satisfy a closure; capture stable handles or shared data.
- Measure before adding caches. A cache must have a clear invalidation owner (and include `rem_size` when geometry is cached).
- Keep animation work bounded and honor reduced motion.

---

## Common Failure Modes

Avoid these anti-patterns:
- One entity containing the entire application's unrelated state;
- Business logic and network requests embedded in a long `render` method;
- Random or index-based `ElementId` values for reorderable content;
- Literal colors and radii that break custom themes;
- Custom clickable `div`s where a semantic component already supplies focus, keyboard, disabled, and accessibility behavior;
- Duplicated local state that drifts from a controlled model value;
- `cx.notify()` loops caused by mutation during every render;
- Nested scroll containers without explicit ownership;
- A new component variant for a one-off screen;
- Confirmation dialogs for reversible, low-risk actions;
- Tests that call internal methods but never exercise keyboard or pointer behavior.
