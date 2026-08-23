# Checklists & Design Review

## Guidance for AI-Generated Interfaces

An AI changing a GPUI interface must:
1. Inspect the nearest feature, theme tokens, and component documentation;
2. State the primary task, state owner, component composition, and keyboard path before generating code;
3. Never infer APIs from React/Shadcn or invent methods by analogy;
4. Ensure human review confirms why hierarchy, density, and tokens belong in the product.

---

## Accessibility Checklist

Before considering a screen complete, verify that:
- [ ] Every action is reachable and operable by keyboard;
- [ ] Focus order follows visual and task order;
- [ ] Focus remains visible and is restored after overlays dismiss;
- [ ] Controls have accessible names, and icon-only controls have tooltips;
- [ ] Text and meaningful boundaries have sufficient contrast;
- [ ] Status is not communicated by color alone;
- [ ] Disabled and read-only states are distinguishable;
- [ ] Labels, errors, and descriptions remain near their controls;
- [ ] Content remains usable with longer translations and larger text/zoom;
- [ ] Pointer targets are comfortably sized even in a dense layout.

---

## 8-Point Design Review Checklist

1. **Is the task clear?** Can a new user recognize the purpose, primary action, and next step without guessing?
2. **Does every action keep its promise?** Do label, control, state, scope, feedback, and result describe one consistent outcome?
3. **Is hierarchy decisive and restrained?** Does the core feature receive space while strong colors, badges, alerts, and primary Buttons remain scarce?
4. **Could the interface do less, better?** Can an entry point or option be removed or deferred?
5. **Is the structure exact?** Do peers share alignment spines, equal gaps stay equal to the rendered pixel, and scrollbars sit at the edge of scrolling regions?
6. **Does it follow the component system?** Do standard controls retain geometry and keyboard behaviors, with appearance from theme and scale tokens?
7. **Does it remain usable in every constraint?** Verify keyboard/focus, empty/loading/error states, zoom, and reduced motion.
8. **Has it been tested in a real window?** Complete the task with real components, copy, and representative content.
