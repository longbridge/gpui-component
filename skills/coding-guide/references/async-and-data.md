# Async Work, Layout, and Large Data

## Async Work and Side Effects

- Start async work from an event, lifecycle hook, or named method—not as an unconditional side effect of `render`.
- Capture weak entities (`cx.weak()`) when work should not keep a closed view alive.
- When the task completes, update state through the GPUI context, handle the case where the entity or window no longer exists, and notify once after the coherent state change.
- Represent async operations with explicit states: `idle`, `loading`, `loaded`, `failed`.
- Preserve usable previous data during refresh when possible. Surface recoverable errors in the UI; do not rely on logs as user feedback.
- Use background executors for expensive parsing/computation, but keep GPUI entity mutation on the appropriate application context. Reject stale work by attaching a revision or identity.

---

## Layout, Measurement, and Scrolling

- Most UI should use GPUI layout rather than measuring itself.
- Put measurement and geometry in the layer that owns the behavior.
- Observe bounds in prepaint only when ordinary layout cannot express the relationship. Never mutate unrelated application state every prepaint.
- Sibling regions should consume the same spacing token or shared inset instead of repeating equivalent literals.

### Scroll Ownership Rules

- Every scrollable region must have **one owner**.
- In flex layouts, apply `min_w_0()` or `min_h_0()` to the flexible child that is allowed to shrink.
- Avoid accidental nested scrolling; route wheel input to the intended axis.
- Attach `Scrollable` to the element that owns the full panel, editor, or window viewport so its scrollbar resolves against the region edge. Put content inset inside that scroll owner rather than wrapping the scroll owner in a padded container.

---

## Lists, Tables, and Large Data

Use virtualization when data can grow beyond a small, bounded collection:
- Separate:
  1. Source data and domain IDs;
  2. Filtering / sorting state;
  3. Selection state;
  4. Viewport / scroll state;
  5. Row rendering.
- Let a stateful list or table own navigation, selection, scroll coordination, and visible-range calculation while item renderers own row presentation.
- Keyboard selection and scroll-to-item must operate in model coordinates even when most elements do not exist in the current frame.
