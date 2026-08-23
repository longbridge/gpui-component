# State Ownership & Notifications

Put each state in the narrowest owner that can keep it correct:
- **domain state** belongs to a model or feature view;
- **transient view state** belongs to the view that renders it;
- **reusable behavioral state** belongs to the component state designed for it;
- **tiny element-local state** may use GPUI keyed element state;
- **shared application services** may be stored as GPUI globals.

## Controlled Values vs Callbacks

Prefer controlled values for ordinary selection and toggles: pass the current value into the component, receive a requested change, update the owner, and render again. A callback reports intent; it should not create a second hidden source of truth.

```rust
Checkbox::new("show-hidden")
    .checked(self.show_hidden)
    .label("Show hidden files")
    .on_click(cx.listener(|this, checked, _, cx| {
        this.show_hidden = *checked;
        cx.notify();
    }))
```

## Notification & Observation Rules

- Call `cx.notify()` after a mutation that changes rendering.
- Use `cx.emit(...)` for a semantic event that an owner should handle.
- Use `cx.subscribe(...)` or `cx.observe(...)` when the lifetime should follow an entity. Keep returned subscriptions alive when the API requires it.
- Do not notify merely because a value was read or derived.
- Avoid unconditional notification from `render`; it schedules another render and can create a permanent redraw loop.
- When several fields form one invariant, update them together and notify once.
- A reusable state type that cannot receive a context should make that limitation explicit and require its owner to emit/notify.

## Avoid State Feedback Loops

Text input, selection, filters, and controlled popups commonly have two paths: an external owner updates the value, and user interaction requests a new value.

- Do not send an owner-supplied value back through the user callback during sync.
- Track the origin or compare coherent snapshots so each logical change is reported once.
- Make callbacks re-entrancy-safe when a callback can synchronously close, replace, or update the component that invoked it.
