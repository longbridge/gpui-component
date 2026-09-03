## Navigation

Load the relevant reference file based on the task:

| Topic                       | File                                    | When to load                                                    |
| --------------------------- | --------------------------------------- | --------------------------------------------------------------- |
| Actions & keybindings       | [action.md](gpui/action.md)             | `actions!`, `bind_keys`, `on_action`, `key_context`             |
| Async & background tasks    | [async.md](gpui/async.md)               | `cx.spawn`, `background_spawn`, `Task`, async I/O               |
| Context management          | [context.md](gpui/context.md)           | `App`, `Window`, `Context<T>`, `AsyncApp`                       |
| Custom elements (low-level) | [element.md](gpui/element.md)           | `Element` trait, `request_layout`, `prepaint`, `paint`          |
| Entity state                | [entity.md](gpui/entity.md)             | `Entity<T>`, `WeakEntity`, state management                     |
| Events & subscriptions      | [event.md](gpui/event.md)               | `cx.emit`, `cx.subscribe`, `cx.observe`                         |
| Focus & keyboard nav        | [focus-handle.md](gpui/focus-handle.md) | `FocusHandle`, `track_focus`, Tab navigation                    |
| Global state                | [global.md](gpui/global.md)             | `Global` trait, `cx.set_global`, app-wide config                |
| Layout & styling            | [layout-style.md](gpui/layout-style.md) | `div()`, `h_flex()`, `v_flex()`, flexbox, overflow, positioning |
| ElementId                   | [element-id.md](gpui/element-id.md)     | `ElementId`, `.id()`, uniqueness rules, stateful elements       |
| Testing                     | [test.md](gpui/test.md)                 | `#[gpui_kit::test]`, `TestAppContext`, `VisualTestContext`          |

## Extended References

For deep-dive topics, additional reference files are available:

**Element trait:**

- [element-api.md](gpui/element-api.md) — complete API, hitbox system, event handling
- [element-patterns.md](gpui/element-patterns.md) — text, interactive, container, composite patterns
- [element-examples.md](gpui/element-examples.md) — full examples: text, interactive, complex elements
- [element-best-practices.md](gpui/element-best-practices.md) — performance, state, common pitfalls
- [element-advanced.md](gpui/element-advanced.md) — masonry/circular layouts, async updates, virtual lists

**Entity management:**

- [entity-api.md](gpui/entity-api.md) — complete Entity API, methods, lifecycle
- [entity-patterns.md](gpui/entity-patterns.md) — model-view, cross-entity communication, observer
- [entity-best-practices.md](gpui/entity-best-practices.md) — memory, performance, lifecycle
- [entity-advanced.md](gpui/entity-advanced.md) — collections, registry, debounce, state machines

**Testing:**

- [test-examples.md](gpui/test-examples.md) — testing examples and patterns
- [test-reference.md](gpui/test-reference.md) — complete testing API reference
