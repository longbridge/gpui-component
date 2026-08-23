# Overlays, Feedback, and Motion

## Choose the Smallest Surface

Choose the smallest surface that fits the decision:
- **Tooltip**: Short explanation or shortcut;
- **Popover**: Contextual controls that do not interrupt the task;
- **Menu**: Compact list of actions;
- **Notification**: Asynchronous status that does not require a decision;
- **Dialog**: Focused decision or short form;
- **Alert Dialog**: Explicit confirmation of a consequential action;
- **Sheet**: Supplementary work needing more space.

## Overlay Rules

- Avoid stacking overlays. Escape should dismiss the topmost dismissible layer, and focus should return to the trigger.
- **Alerts vs Toasts**: Use Alert for important, exceptional information requiring attention in current task. Do not use Alert for routine descriptions or tips.
- **Footer rules**: Footer actions must have clear objects and scopes. Do not place ambiguous icons in footers.

---

## Motion & Transitions

- Motion explains change; it is not ambient decoration.
- Use short transitions for appearance, dismissal, expansion, and spatial continuity.
- Honor reduced-motion preferences (`window.rem_size()` / system settings).
- Never require animation to understand state.
- Make interruptions reverse smoothly from the currently sampled value rather than restarting from old endpoints.
